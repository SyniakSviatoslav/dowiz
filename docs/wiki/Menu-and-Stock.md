# Menu and stock

## The menu

The owner edits the menu in the console's **Menu** tab.

- **Categories and dishes**, with a price, a photo, and a name and description in Albanian, English
  and Ukrainian. A dish missing a translation shows in the venue's own language.
- **Allergens** and **options** (modifiers, such as a size or an extra) per dish.
- **Availability**: a dish marked sold out stays on the menu but cannot be added to a cart.
- **Photos** are stored once, by the hash of their content, and served from the venue's own address.
- **Import from a spreadsheet.** A CSV becomes dishes and categories. The first run is always a
  **preview** that lists what would change and every row that could not be read; nothing is written
  until the owner applies it. Dishes missing from the file can be retired in the same step.

Prices are integers in the currency's smallest unit (for the lek, whole lek). A price never passes
through a floating-point number anywhere in the system.

## Stock

The **Stock** tab is a ledger of what the kitchen buys and uses.

- **Supplies**: the things the kitchen buys, each with its unit. Retiring a supply stops tracking it;
  it is not deleted, and its history stays.
- **Recipes**: each dish's lines (how much of which supply). A dish's weight, energy (approximate
  kcal) and cost follow from its lines. A dish's cost is the weighted average of what its supplies
  were bought for.
- **Purchases** add to a supply; **write-offs** take away, and each names who signed it.
- **Waste report**: what was written off, and why.
- **Import supplies and recipes** from a spreadsheet, with the same preview-then-apply rule as the
  menu. The spreadsheet's headers can be in any of the three languages, and `1.200,00` and `1200` are
  read as the same number.

**The ledger does nothing until a venue feeds it.** It is built and tested end to end, but an order
only reserves stock for a dish that has a recipe. A venue with no supplies and no recipes sees no
effect on its orders.
