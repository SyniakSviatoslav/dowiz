// @ts-nocheck
import type { MenuParserProvider, ParserInputType } from '../ports.js';
import type { CanonicalMenuDraft, ParseIssue, ParseResult, ParseMode, CanonicalCategory, CanonicalProduct, CanonicalModifierGroup, CanonicalModifier } from '@deliveryos/shared-types';

export class CsvMenuParser implements MenuParserProvider {
  readonly id = 'csv';

  async parse(input: ParserInputType): Promise<ParseResult> {
    if (input.kind !== 'csv') throw new Error('Unsupported input kind');
    
    const text = input.bytes.toString('utf-8');
    const { delimiter = this.detectDelimiter(text), expectedCurrency, currencyMinorUnit = 0, columnMap = {} } = input.config;
    
    const rows = this.parseCsv(text, delimiter);
    if (rows.length === 0) {
      return {
        draft: this.emptyDraft(),
        issues: [{ rowNumber: 1, code: 'EMPTY_ROW', message: 'File is empty', severity: 'error' }],
        summary: { valid: 0, errors: 1, warnings: 0, mode: 'merge' }
      };
    }

    const header = rows[0].map(h => h.trim().toLowerCase());
    const dataRows = rows.slice(1);
    
    const draft = this.emptyDraft();
    const issues: ParseIssue[] = [];

    const getCol = (row: string[], colName: string) => {
      const mapped = columnMap[colName] || colName;
      const idx = header.indexOf(mapped);
      return idx >= 0 ? row[idx] : undefined;
    };

    let validCount = 0;
    let errorsCount = 0;
    let warningsCount = 0;

    const addIssue = (rowNumber: number, code: ParseIssue['code'], message: string, severity: 'error'|'warning', raw?: string) => {
      issues.push({ rowNumber, code, message, severity, raw });
      if (severity === 'error') errorsCount++;
      if (severity === 'warning') warningsCount++;
    };

    const categoriesMap = new Map<string, CanonicalCategory>();
    const productsMap = new Map<string, CanonicalProduct>();
    const modifierGroupsMap = new Map<string, CanonicalModifierGroup>();
    const modifiersMap = new Map<string, CanonicalModifier>();
    const linksSet = new Set<string>();

    for (let i = 0; i < dataRows.length; i++) {
      const row = dataRows[i];
      const rowNumber = i + 2; // +1 for 0-index, +1 for header
      
      // Skip empty rows
      if (row.length === 0 || (row.length === 1 && row[0].trim() === '')) {
        continue;
      }

      // Check CSV injection guard
      let hasInjection = false;
      for (const val of row) {
        if (/^[=+\-@|]/.test(val)) {
          addIssue(rowNumber, 'POTENTIALLY_UNSAFE_VALUE', `Value starts with unsafe character: ${val.substring(0, 10)}...`, 'error', val);
          hasInjection = true;
          break;
        }
      }
      if (hasInjection) continue;

      const catKey = getCol(row, 'category_key')?.trim();
      const catName = getCol(row, 'category_name')?.trim();
      const prodKey = getCol(row, 'product_key')?.trim();
      const prodName = getCol(row, 'product_name')?.trim();
      const prodDesc = getCol(row, 'product_description')?.trim();
      const priceStr = getCol(row, 'price')?.trim();
      const currency = getCol(row, 'currency')?.trim();
      const prodAvailableStr = getCol(row, 'available')?.trim();
      const attrJsonStr = getCol(row, 'attributes_json')?.trim();
      const imageKey = getCol(row, 'image_key')?.trim();

      const modGroupKey = getCol(row, 'modifier_group_key')?.trim();
      const modGroupName = getCol(row, 'modifier_group_name')?.trim();
      const minSelectStr = getCol(row, 'min_select')?.trim();
      const maxSelectStr = getCol(row, 'max_select')?.trim();
      const requiredStr = getCol(row, 'required')?.trim();

      const modKey = getCol(row, 'modifier_key')?.trim();
      const modName = getCol(row, 'modifier_name')?.trim();
      const modPriceStr = getCol(row, 'modifier_price_delta')?.trim();
      const modAvailableStr = getCol(row, 'modifier_available')?.trim();
      const modSortStr = getCol(row, 'modifier_sort_order')?.trim();

      if (!prodKey) {
        addIssue(rowNumber, 'MISSING_REQUIRED', 'product_key is required', 'error');
        continue;
      }

      // Products & Categories
      if (catKey && catName && !categoriesMap.has(catKey)) {
        categoriesMap.set(catKey, { externalKey: catKey, name: catName });
      }

      let rowHasProductError = false;

      if (!productsMap.has(prodKey)) {
        if (!catKey || !prodName || !priceStr || !currency) {
          addIssue(rowNumber, 'MISSING_REQUIRED', 'Missing category, name, price or currency for new product', 'error');
          rowHasProductError = true;
        } else if (expectedCurrency && currency.toUpperCase() !== expectedCurrency.toUpperCase()) {
          addIssue(rowNumber, 'CURRENCY_MISMATCH', `Currency ${currency} does not match expected ${expectedCurrency}`, 'error');
          rowHasProductError = true;
        } else {
          const parsedPrice = this.parsePrice(priceStr, currencyMinorUnit);
          if (parsedPrice === null) {
            addIssue(rowNumber, 'INVALID_PRICE', `Invalid price format: ${priceStr}`, 'error');
            rowHasProductError = true;
          } else {
            let attrJson: Record<string, unknown> | undefined;
            if (attrJsonStr) {
              try {
                attrJson = JSON.parse(attrJsonStr);
              } catch {
                addIssue(rowNumber, 'PARSE_ERROR', 'attributes_json is invalid', 'warning');
              }
            }

            productsMap.set(prodKey, {
              externalKey: prodKey,
              categoryKey: catKey!,
              name: prodName!,
              description: prodDesc,
              price: parsedPrice,
              currency: currency.toUpperCase(),
              available: this.parseBoolean(prodAvailableStr, true),
              attributesJson: attrJson,
              imageKey
            });
            validCount++;
          }
        }
      }

      if (rowHasProductError) continue;

      // Modifiers
      if (modGroupKey) {
        if (!modifierGroupsMap.has(modGroupKey)) {
          if (!modGroupName) {
            addIssue(rowNumber, 'MISSING_REQUIRED', 'modifier_group_name is required for new group', 'error');
            continue;
          }
          modifierGroupsMap.set(modGroupKey, {
            externalKey: modGroupKey,
            name: modGroupName,
            minSelect: parseInt(minSelectStr || '0', 10) || 0,
            maxSelect: parseInt(maxSelectStr || '1', 10) || 1,
            required: this.parseBoolean(requiredStr, false)
          });
        }

        const linkKey = `${prodKey}::${modGroupKey}`;
        if (!linksSet.has(linkKey)) {
          linksSet.add(linkKey);
          draft.links.push({ productKey: prodKey, groupKey: modGroupKey, sortOrder: draft.links.length });
        }

        if (modKey) {
          const modGlobalKey = `${modGroupKey}::${modKey}`;
          if (!modifiersMap.has(modGlobalKey)) {
            if (!modName || !modPriceStr) {
              addIssue(rowNumber, 'MISSING_REQUIRED', 'modifier_name and modifier_price_delta are required', 'error');
            } else {
              const parsedModPrice = this.parsePrice(modPriceStr, currencyMinorUnit);
              if (parsedModPrice === null) {
                addIssue(rowNumber, 'INVALID_MODIFIER_PRICE_DELTA', `Invalid modifier price: ${modPriceStr}`, 'error');
              } else {
                modifiersMap.set(modGlobalKey, {
                  externalKey: modKey,
                  groupKey: modGroupKey,
                  name: modName,
                  priceDelta: parsedModPrice,
                  available: this.parseBoolean(modAvailableStr, true),
                  sortOrder: parseInt(modSortStr || '0', 10) || 0
                });
              }
            }
          }
        }
      }
    }

    draft.categories = Array.from(categoriesMap.values());
    draft.products = Array.from(productsMap.values());
    draft.modifierGroups = Array.from(modifierGroupsMap.values());
    draft.modifiers = Array.from(modifiersMap.values());

    return {
      draft,
      issues,
      summary: { valid: validCount, errors: errorsCount, warnings: warningsCount, mode: 'merge' }
    };
  }

  private detectDelimiter(text: string): ',' | ';' | '\t' {
    const firstLine = text.split('\n')[0] || '';
    const commas = (firstLine.match(/,/g) || []).length;
    const semis = (firstLine.match(/;/g) || []).length;
    const tabs = (firstLine.match(/\t/g) || []).length;
    if (tabs > commas && tabs > semis) return '\t';
    if (semis > commas && semis > tabs) return ';';
    return ',';
  }

  private parseBoolean(val: string | undefined, def: boolean): boolean {
    if (!val) return def;
    const lower = val.toLowerCase();
    if (['1', 'true', 'yes', 'y'].includes(lower)) return true;
    if (['0', 'false', 'no', 'n'].includes(lower)) return false;
    return def;
  }

  private parsePrice(val: string, minorUnit: number): number | null {
    const num = parseFloat(val);
    if (isNaN(num)) return null;
    const multiplier = Math.pow(10, minorUnit);
    // Half-up rounding logic for fractional resolution
    return Math.round(num * multiplier);
  }

  private emptyDraft(): CanonicalMenuDraft {
    return {
      categories: [],
      products: [],
      modifierGroups: [],
      modifiers: [],
      links: [],
      translations: []
    };
  }

  private parseCsv(text: string, delimiter: string): string[][] {
    const rows: string[][] = [];
    let currentRow: string[] = [];
    let currentCell = '';
    let inQuotes = false;

    for (let i = 0; i < text.length; i++) {
      const char = text[i];
      const nextChar = text[i + 1];

      if (inQuotes) {
        if (char === '"') {
          if (nextChar === '"') {
            currentCell += '"';
            i++; // skip next quote
          } else {
            inQuotes = false;
          }
        } else {
          currentCell += char;
        }
      } else {
        if (char === '"') {
          inQuotes = true;
        } else if (char === delimiter) {
          currentRow.push(currentCell);
          currentCell = '';
        } else if (char === '\r' && nextChar === '\n') {
          currentRow.push(currentCell);
          rows.push(currentRow);
          currentRow = [];
          currentCell = '';
          i++; // skip \n
        } else if (char === '\n' || char === '\r') {
          currentRow.push(currentCell);
          rows.push(currentRow);
          currentRow = [];
          currentCell = '';
        } else {
          currentCell += char;
        }
      }
    }
    if (currentCell || currentRow.length > 0) {
      currentRow.push(currentCell);
      rows.push(currentRow);
    }
    return rows;
  }
}
