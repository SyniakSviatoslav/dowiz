// ─── dowiz — Complete Frontend Application ───
// Built on original mobile-first restaurant design

const App = {
  cart: [],
  orders: JSON.parse(localStorage.getItem('dowiz-orders') || '[]'),
  currentCat: 'pizza',
  views: ['menu', 'cart', 'orders', 'admin'],

  menu: {
    pizza: [
      { id: 1, name: 'Маргарита', desc: 'Томатний соус, моцарела фіор ді латте, свіжий базилік', price: 850, emoji: '\u{1F355}', bg: 'card-thumb-margherita', avail: true },
      { id: 2, name: 'Пепероні', desc: 'Гостра салямі, моцарела, томатний соус, орегано', price: 950, emoji: '\u{1F355}', bg: 'card-thumb-pepperoni', avail: false },
      { id: 3, name: 'Чотири сири', desc: 'Горгонзола, моцарела, пармезан, таледжо та трюфельний мед', price: 1050, emoji: '\u{1F9C0}', bg: 'card-thumb-formaggi', avail: true },
    ],
    pasta: [
      { id: 4, name: 'Паста Карбонара', desc: 'Спагеті, гуанчале, яєчний жовток, пекоріно романо', price: 780, emoji: '\u{1F35D}', bg: 'card-thumb-carbonara', avail: true },
      { id: 5, name: 'Болоньєзе', desc: 'Спагеті, м\'ясний соус, пармезан, свіжа петрушка', price: 820, emoji: '\u{1F35D}', bg: 'card-thumb-formaggi', avail: true },
    ],
    salads: [
      { id: 6, name: 'Салат Цезар', desc: 'Ромен, курка гриль, пармезан, крутони, класичний соус', price: 620, emoji: '\u{1F957}', bg: 'card-thumb-caesar', avail: true },
      { id: 7, name: 'Грецький салат', desc: 'Помідори, огірок, фета, оливки, цибуля, орегано', price: 540, emoji: '\u{1F957}', bg: 'card-thumb-caesar', avail: true },
    ],
    drinks: [
      { id: 8, name: 'Домашній лимонад', desc: 'Свіжий лимон, м\'ята, газована вода, тростинний цукор', price: 320, emoji: '\u{1F9CB}', bg: 'card-thumb-lemonade', avail: true },
      { id: 9, name: 'Айран', desc: 'Кисломолочний напій, сіль, зелена цибуля', price: 180, emoji: '\u{1F95B}', bg: 'card-thumb-lemonade', avail: true },
    ],
    desserts: [
      { id: 10, name: 'Тірамісу', desc: 'Кавові бісквіти, маскарпоне, какао, еспресо', price: 450, emoji: '\u{1F370}', bg: 'card-thumb-formaggi', avail: true },
      { id: 11, name: 'Панна-котта', desc: 'Вершковий десерт, ягідний соус, свіжа м\'ята', price: 380, emoji: '\u{1F36B}', bg: 'card-thumb-lemonade', avail: true },
    ]
  },

  init() {
    this.renderMenu();
    this.bindEvents();
    this.renderCartFab();
    this.refreshOrders();
  },

  renderMenu() {
    const items = this.menu[this.currentCat] || [];
    const labels = { pizza:'Піца', pasta:'Паста', salads:'Салати', drinks:'Напої', desserts:'Десерти' };
    const section = document.querySelector('.section-title');
    const tag = document.querySelector('.section-tag');
    const grid = document.querySelector('.product-grid');

    if (section) section.textContent = labels[this.currentCat] || 'Меню';
    if (tag) tag.textContent = `${items.length} позицій`;

    if (grid) {
      grid.innerHTML = items.map(item => `
        <article class="card" aria-label="${item.name}, ${item.price} ALL">
          <div class="card-photo">
            <div class="card-thumb ${item.bg}">
              <span role="img" aria-label="${item.name}">${item.emoji}</span>
            </div>
            ${!item.avail ? '<div class="unavail-overlay" aria-hidden="true"><span class="unavail-label">Тимчасово недоступно</span></div>' : ''}
          </div>
          <div class="card-body">
            <p class="card-name">${item.name}</p>
            <p class="card-desc">${item.desc}</p>
            <div class="card-footer">
              <span class="card-price ${!item.avail ? 'card-price--muted' : ''}">${item.price.toLocaleString()} ALL</span>
              <button class="add-btn" ${!item.avail ? 'disabled aria-label="Недоступно"' : `onclick="App.addToCart(${item.id})" aria-label="Додати ${item.name}"`}>+</button>
            </div>
          </div>
        </article>
      `).join('');
    }
  },

  addToCart(id) {
    for (const cat of Object.values(this.menu)) {
      const item = cat.find(i => i.id === id);
      if (item) {
        const existing = this.cart.find(i => i.id === id);
        if (existing) existing.qty++;
        else this.cart.push({ ...item, qty: 1 });
        this.renderCartFab();
        this.animateAdd(id);
        return;
      }
    }
  },

  animateAdd(id) {
    const btn = document.querySelector(`.add-btn[onclick*="${id}"]`);
    if (btn) {
      btn.textContent = '\u2713';
      btn.style.background = '#059669';
      setTimeout(() => { btn.textContent = '+'; btn.style.background = ''; }, 700);
    }
  },

  renderCartFab() {
    const count = this.cart.reduce((s, i) => s + i.qty, 0);
    const total = this.cart.reduce((s, i) => s + i.price * i.qty, 0);
    const fab = document.querySelector('.fab-count');
    const price = document.querySelector('.fab-price');
    if (fab) fab.textContent = `${count} ${this.plural(count, 'позиція', 'позиції', 'позицій')}`;
    if (price) price.textContent = `${total.toLocaleString()} ALL`;
    const badge = document.querySelector('.cart-badge');
    if (badge) badge.textContent = count;
  },

  plural(n, one, few, many) {
    n = Math.abs(n) % 100;
    if (n > 10 && n < 20) return many;
    n = n % 10;
    if (n === 1) return one;
    if (n > 1 && n < 5) return few;
    return many;
  },

  showView(view) {
    document.querySelectorAll('.view').forEach(v => v.classList.remove('view-active'));
    const el = document.getElementById(`view-${view}`);
    if (el) el.classList.add('view-active');
    if (view === 'orders') this.renderOrders();
    if (view === 'admin') this.renderAdmin();
  },

  renderOrders() {
    const el = document.getElementById('orders-list');
    if (!el) return;
    el.innerHTML = this.orders.length === 0
      ? '<p style="text-align:center;padding:48px 0;color:var(--brand-text-muted)">Немає активних замовлень</p>'
      : this.orders.map(o => `
        <div class="order-item" style="display:flex;justify-content:space-between;align-items:center;padding:14px 0;border-bottom:1px solid var(--brand-border)">
          <div>
            <div style="font-weight:600">#${o.id}</div>
            <div style="font-size:12px;color:var(--brand-text-muted)">${o.items} позицій · ${o.total.toLocaleString()} ALL</div>
          </div>
          <span style="font-size:11px;font-weight:600;padding:3px 10px;border-radius:999px;background:${o.status === 'delivered' ? '#059669' : '#D97706'};color:#fff">${o.status === 'delivered' ? 'Доставлено' : 'В обробці'}</span>
        </div>
      `).join('');
  },

  renderAdmin() {
    const el = document.getElementById('admin-stats');
    if (!el) return;
    const totalOrders = this.orders.length;
    const revenue = this.orders.reduce((s, o) => s + o.total, 0);
    el.innerHTML = `
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:12px;margin-bottom:24px">
        <div style="text-align:center;padding:20px;background:var(--brand-surface);border-radius:var(--brand-radius)">
          <div style="font-size:28px;font-weight:700;color:var(--brand-primary)">${totalOrders}</div>
          <div style="font-size:12px;color:var(--brand-text-muted)">Всього замовлень</div>
        </div>
        <div style="text-align:center;padding:20px;background:var(--brand-surface);border-radius:var(--brand-radius)">
          <div style="font-size:28px;font-weight:700;color:var(--brand-primary)">${revenue.toLocaleString()} ALL</div>
          <div style="font-size:12px;color:var(--brand-text-muted)">Дохід</div>
        </div>
      </div>
      <div style="background:var(--brand-surface);border-radius:var(--brand-radius);padding:20px">
        <h4 style="margin-bottom:12px">Останні замовлення</h4>
        ${this.orders.length === 0 ? '<p style="color:var(--brand-text-muted);text-align:center;padding:24px">Немає замовлень</p>' :
          this.orders.slice(-5).reverse().map(o => `
            <div style="display:flex;justify-content:space-between;padding:8px 0;border-bottom:1px solid var(--brand-border);font-size:13px">
              <span>#${o.id}</span>
              <span style="color:var(--brand-text-muted)">${o.total.toLocaleString()} ALL</span>
            </div>`).join('')}
      </div>`;
  },

  refreshOrders() {
    localStorage.setItem('dowiz-orders', JSON.stringify(this.orders));
  },

  checkout() {
    if (this.cart.length === 0) return;
    const order = {
      id: Date.now() % 100000,
      items: this.cart.reduce((s, i) => s + i.qty, 0),
      total: this.cart.reduce((s, i) => s + i.price * i.qty, 0),
      date: new Date().toISOString(),
      status: 'pending'
    };
    this.orders.push(order);
    this.refreshOrders();
    this.cart = [];
    this.renderCartFab();
    this.showView('orders');
  },

  bindEvents() {
    // Category navigation
    document.querySelectorAll('.cat-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        document.querySelectorAll('.cat-btn').forEach(b => {
          b.classList.remove('active');
          b.setAttribute('aria-selected', 'false');
        });
        btn.classList.add('active');
        btn.setAttribute('aria-selected', 'true');
        this.currentCat = btn.dataset.cat;
        this.renderMenu();
      });
    });

    // Fab button
    document.querySelector('.fab')?.addEventListener('click', () => this.showView('cart'));

    // Bottom navigation
    document.querySelectorAll('.nav-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        document.querySelectorAll('.nav-btn').forEach(b => b.classList.remove('nav-active'));
        btn.classList.add('nav-active');
        this.showView(btn.dataset.view);
      });
    });
  }
};

document.addEventListener('DOMContentLoaded', () => App.init());
window.App = App;
