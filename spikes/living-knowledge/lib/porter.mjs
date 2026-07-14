// porter.mjs — compact Porter stemmer (the classic public-domain algorithm). Deterministic, zero-dep.
// Purpose: give the lexical (BM25) signal morphology-robustness so a query term matches its inflections
// (classified/classification/classify → classif*, reference/references → refer, operating/operations →
// oper). Without it, "classified" (query) never matches "classification" (doc) — a real recall bug this
// engine hit. Pure function of the input string → keeps the whole retriever deterministic.
const step2list = { ational: 'ate', tional: 'tion', enci: 'ence', anci: 'ance', izer: 'ize', bli: 'ble', alli: 'al', entli: 'ent', eli: 'e', ousli: 'ous', ization: 'ize', ation: 'ate', ator: 'ate', alism: 'al', iveness: 'ive', fulness: 'ful', ousness: 'ous', aliti: 'al', iviti: 'ive', biliti: 'ble', logi: 'log' };
const step3list = { icate: 'ic', ative: '', alize: 'al', iciti: 'ic', ical: 'ic', ful: '', ness: '' };
const c = '[^aeiou]', v = '[aeiouy]', C = c + '[^aeiouy]*', V = v + '[aeiou]*';
const mgr0 = new RegExp('^(' + C + ')?' + V + C), meq1 = new RegExp('^(' + C + ')?' + V + C + '(' + V + ')?$'), mgr1 = new RegExp('^(' + C + ')?' + V + C + V + C), s_v = new RegExp('^(' + C + ')?' + v);
export function stem(w) {
  if (w.length < 3) return w;
  let stem, suffix, re, re2, re3, re4, firstch = w[0];
  if (firstch === 'y') w = firstch.toUpperCase() + w.substr(1);
  re = /^(.+?)(ss|i)es$/; re2 = /^(.+?)([^s])s$/;
  if (re.test(w)) w = w.replace(re, '$1$2'); else if (re2.test(w)) w = w.replace(re2, '$1$2');
  re = /^(.+?)eed$/; re2 = /^(.+?)(ed|ing)$/;
  if (re.test(w)) { const fp = re.exec(w); if (mgr0.test(fp[1])) w = w.replace(/.$/, ''); }
  else if (re2.test(w)) { const fp = re2.exec(w); stem = fp[1]; if (s_v.test(stem)) { w = stem; re2 = /(at|bl|iz)$/; re3 = new RegExp('([^aeiouylsz])\\1$'); re4 = new RegExp('^' + C + v + '[^aeiouwxy]$'); if (re2.test(w)) w = w + 'e'; else if (re3.test(w)) w = w.replace(/.$/, ''); else if (re4.test(w)) w = w + 'e'; } }
  re = /^(.+?)y$/; if (re.test(w)) { const fp = re.exec(w); stem = fp[1]; if (s_v.test(stem)) w = stem + 'i'; }
  re = new RegExp('^(.+?)(' + Object.keys(step2list).join('|') + ')$'); if (re.test(w)) { const fp = re.exec(w); stem = fp[1]; suffix = fp[2]; if (mgr0.test(stem)) w = stem + step2list[suffix]; }
  re = new RegExp('^(.+?)(' + Object.keys(step3list).join('|') + ')$'); if (re.test(w)) { const fp = re.exec(w); stem = fp[1]; suffix = fp[2]; if (mgr0.test(stem)) w = stem + step3list[suffix]; }
  re = /^(.+?)(al|ance|ence|er|ic|able|ible|ant|ement|ment|ent|ou|ism|ate|iti|ous|ive|ize)$/; re2 = /^(.+?)(s|t)(ion)$/;
  if (re.test(w)) { const fp = re.exec(w); stem = fp[1]; if (mgr1.test(stem)) w = stem; }
  else if (re2.test(w)) { const fp = re2.exec(w); stem = fp[1] + fp[2]; if (mgr1.test(stem)) w = stem; }
  re = /^(.+?)e$/; if (re.test(w)) { const fp = re.exec(w); stem = fp[1]; re3 = new RegExp('^' + C + v + '[^aeiouwxy]$'); if (mgr1.test(stem) || (meq1.test(stem) && !re3.test(stem))) w = stem; }
  re = /ll$/; if (re.test(w) && mgr1.test(w)) w = w.replace(/.$/, '');
  if (firstch === 'y') w = firstch.toLowerCase() + w.substr(1);
  return w;
}
