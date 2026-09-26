// The words of the "AI agent (MCP)" panel (/lib/mcp.js), in the three
// languages every app speaks. ONE copy for the three apps: the panel is the
// same panel, and three dictionaries would drift apart by the next edit.
// Plain ASCII quotes only (a typographic quote once took the console down).

const WORDS = {
  sq: {
    title: 'Agjenti AI (MCP)',
    hintOwner: 'Lidhni Claude, Codex ose cilindo agjent MCP me lokalin. Agjenti merr vetëm veglat e çelësit: çelësi juaj API jep veglat e pronarit; çdo punonjës dhe korrier krijon çelësin e vet në aplikacionin e vet dhe merr vetëm të drejtat e veta.',
    hintStaff: 'Lidhni Claude, Codex ose cilindo agjent MCP me punën tuaj. Çelësi që krijoni këtu i jep agjentit vetëm atë që mund të bëni ju, dhe ndalon kur ju ndalon roli ose e revokoni.',
    hintCourier: 'Lidhni Claude, Codex ose cilindo agjent MCP me turnin tuaj. Agjenti sheh vetëm porositë tuaja dhe ato të lira, si ju në aplikacion.',
    copy: 'Kopjo', copyKey: 'Kopjo çelësin', copied: 'U kopjua',
    keyHow: 'Krijoni një çelës për agjentin', label: 'Për çfarë është (p.sh. laptopi)', mint: 'Krijo çelësin',
    shownOnce: 'Ky çelës shfaqet vetëm tani. Kopjojeni; më pas nuk mund të shihet më.',
    placeholderNote: 'Zëvendësoni {k} me çelësin tuaj.',
    yourKeys: 'Çelësat tuaj', noKeys: 'Asnjë çelës ende.', revoke: 'Revoko', revoked: 'Çelësi u revokua', until: 'deri më',
    tools: 'vegla', noTools: 'Ky rol nuk ka vegla.', clients: 'Si ta lidhni',
    claudeCode: 'Claude Code', claudeDesktop: 'Claude Desktop', codex: 'Codex', generic: 'Çdo klient MCP',
    hintClaudeCode: 'Në terminal, një herë:', hintClaudeDesktop: 'Në claude_desktop_config.json (kërkon Node.js):',
    hintCodex: 'OpenAI Codex (CLI ose IDE), në ~/.codex/config.toml:', hintGeneric: 'Për çdo klient tjetër që flet MCP:',
    roleOf: 'Veglat e rolit', personKeys: 'Çelësat e personave', ownerKey: 'Çelësi juaj: API keys', error: 'Diçka nuk shkoi',
    roleNames: { owner: 'Pronari', waiter: 'Kamarieri', kitchen: 'Kuzhina', 'counter-manager': 'Banakieri', courier: 'Korrieri' },
  },
  en: {
    title: 'AI agent (MCP)',
    hintOwner: 'Connect Claude, Codex or any MCP agent to the venue. An agent gets only its key\'s tools: your API key gives the owner\'s tools; each member of staff and courier mints their own key in their own app and gets only their own rights.',
    hintStaff: 'Connect Claude, Codex or any MCP agent to your work. The key you make here lets the agent do only what you can, and stops when your role does or when you revoke it.',
    hintCourier: 'Connect Claude, Codex or any MCP agent to your shift. The agent sees only your orders and the free ones, as you do in the app.',
    copy: 'Copy', copyKey: 'Copy key', copied: 'Copied',
    keyHow: 'Make a key for your agent', label: 'What it is for (e.g. laptop)', mint: 'Make key',
    shownOnce: 'This key is shown only now. Copy it; it cannot be shown again.',
    placeholderNote: 'Replace {k} with your key.',
    yourKeys: 'Your keys', noKeys: 'No keys yet.', revoke: 'Revoke', revoked: 'Key revoked', until: 'until',
    tools: 'tools', noTools: 'This role has no tools.', clients: 'How to connect',
    claudeCode: 'Claude Code', claudeDesktop: 'Claude Desktop', codex: 'Codex', generic: 'Any MCP client',
    hintClaudeCode: 'In a terminal, once:', hintClaudeDesktop: 'In claude_desktop_config.json (needs Node.js):',
    hintCodex: 'OpenAI Codex (CLI or IDE), in ~/.codex/config.toml:', hintGeneric: 'For any other client that speaks MCP:',
    roleOf: 'Tools of the role', personKeys: 'People\'s keys', ownerKey: 'Your key: API keys', error: 'Something went wrong',
    roleNames: { owner: 'Owner', waiter: 'Waiter', kitchen: 'Kitchen', 'counter-manager': 'Counter', courier: 'Courier' },
  },
  uk: {
    title: 'AI-агент (MCP)',
    hintOwner: 'Підключіть Claude, Codex чи будь-який MCP-агент до закладу. Агент отримує лише інструменти свого ключа: ваш API-ключ дає інструменти власника; кожен працівник і кур\'єр створює власний ключ у своєму застосунку й отримує лише свої права.',
    hintStaff: 'Підключіть Claude, Codex чи будь-який MCP-агент до своєї роботи. Ключ, створений тут, дозволяє агенту лише те, що можете ви, і перестає діяти разом із вашою роллю або коли ви його відкличете.',
    hintCourier: 'Підключіть Claude, Codex чи будь-який MCP-агент до своєї зміни. Агент бачить лише ваші замовлення та вільні, як ви в застосунку.',
    copy: 'Копіювати', copyKey: 'Копіювати ключ', copied: 'Скопійовано',
    keyHow: 'Створіть ключ для агента', label: 'Для чого він (напр. ноутбук)', mint: 'Створити ключ',
    shownOnce: 'Цей ключ видно лише зараз. Скопіюйте його; показати його знову неможливо.',
    placeholderNote: 'Замініть {k} своїм ключем.',
    yourKeys: 'Ваші ключі', noKeys: 'Ключів ще немає.', revoke: 'Відкликати', revoked: 'Ключ відкликано', until: 'до',
    tools: 'інструментів', noTools: 'У цієї ролі немає інструментів.', clients: 'Як підключити',
    claudeCode: 'Claude Code', claudeDesktop: 'Claude Desktop', codex: 'Codex', generic: 'Будь-який MCP-клієнт',
    hintClaudeCode: 'У терміналі, один раз:', hintClaudeDesktop: 'У claude_desktop_config.json (потрібен Node.js):',
    hintCodex: 'OpenAI Codex (CLI чи IDE), у ~/.codex/config.toml:', hintGeneric: 'Для будь-якого іншого клієнта з MCP:',
    roleOf: 'Інструменти ролі', personKeys: 'Ключі людей', ownerKey: 'Ваш ключ: API-ключі', error: 'Щось пішло не так',
    roleNames: { owner: 'Власник', waiter: 'Офіціант', kitchen: 'Кухня', 'counter-manager': 'Каса', courier: 'Кур\'єр' },
  },
};

export const LANGS = Object.keys(WORDS);

/// The panel's words for `lang` (English when unknown), with `hint` set for
/// whose panel it is: 'owner' | 'staff' | 'courier'.
export function mcpWords(lang, who){
  const w = WORDS[lang] || WORDS.en;
  const hint = { owner: w.hintOwner, staff: w.hintStaff, courier: w.hintCourier }[who] || w.hintStaff;
  return { ...w, hint };
}
