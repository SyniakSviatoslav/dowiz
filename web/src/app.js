// ─── dowiz — Complete Interface with Neural Field + Navigation ───

const App = {
  state: {
    theme: localStorage.getItem('dowiz-theme') || 'crimson',
    cart: [],
    page: 'menu',
    filter: 'all',
    _stats: { orders: 1247, nodes: 342, uptime: '99.97%', tests: 1949 },
    _orders: [
      { id: 1001, status: 'pending', items: 3, total: 2680, time: '2 хв' },
      { id: 1002, status: 'confirmed', items: 1, total: 950, time: '8 хв' },
      { id: 1003, status: 'preparing', items: 5, total: 4130, time: '15 хв' },
      { id: 1004, status: 'ready', items: 2, total: 1640, time: '22 хв' },
      { id: 1005, status: 'in-delivery', items: 4, total: 3400, time: '35 хв' },
      { id: 1006, status: 'delivered', items: 2, total: 1230, time: '1 год' },
    ],
    _menu: [
      { id: 1, name: 'Маргарита', price: 850, cat: 'Піца', desc: 'Томатний соус, моцарела фіор ді латте, свіжий базилік', emoji: '\u{1F355}', prep: '15 хв' },
      { id: 2, name: 'Пепероні', price: 950, cat: 'Піца', desc: 'Гостра салямі, моцарела, томатний соус, орегано', emoji: '\u{1F355}', prep: '15 хв' },
      { id: 3, name: 'Чотири сири', price: 1050, cat: 'Піца', desc: 'Горгонзола, моцарела, пармезан, таледжо, трюфельний мед', emoji: '\u{1F9C0}', prep: '18 хв' },
      { id: 4, name: 'Карбонара', price: 780, cat: 'Паста', desc: 'Спагеті, гуанчале, яєчний жовток, пекоріно романо', emoji: '\u{1F35D}', prep: '12 хв' },
      { id: 5, name: 'Болоньєзе', price: 820, cat: 'Паста', desc: 'Спагеті, м\'ясний соус, пармезан, свіжа петрушка', emoji: '\u{1F35D}', prep: '14 хв' },
      { id: 6, name: 'Цезар', price: 620, cat: 'Салати', desc: 'Ромен, курка гриль, пармезан, крутони, класичний соус', emoji: '\u{1F957}', prep: '8 хв' },
      { id: 7, name: 'Грецький', price: 540, cat: 'Салати', desc: 'Помідори, огірок, фета, оливки, цибуля, орегано', emoji: '\u{1F957}', prep: '6 хв' },
      { id: 8, name: 'Домашній лимонад', price: 320, cat: 'Напої', desc: 'Свіжий лимон, м\'ята, газована вода, тростинний цукор', emoji: '\u{1F9CB}', prep: '3 хв' },
      { id: 9, name: 'Айран', price: 180, cat: 'Напої', desc: 'Кисломолочний напій, сіль, зелена цибуля', emoji: '\u{1F95B}', prep: '2 хв' },
      { id: 10, name: 'Тірамісу', price: 450, cat: 'Десерти', desc: 'Кавові бісквіти, маскарпоне, какао, еспресо', emoji: '\u{1F370}', prep: '5 хв' },
      { id: 11, name: 'Панна-котта', price: 380, cat: 'Десерти', desc: 'Вершковий десерт, ягідний соус, свіжа м\'ята', emoji: '\u{1F36B}', prep: '5 хв' },
    ],
    _neurons: null,
    _audioCtx: null,
    _scene: null,
    _spikeCount: 0,
    _frameCount: 0,
    _spikeRate: 0
  },

  async init() {
    document.documentElement.dataset.theme = this.state.theme;
    await this.initNeuralField();
    this.initAudio();
    this.render();
    this.bindEvents();
  },

  // ═══ Three.js Neural Field ═══

  async initNeuralField() {
    const canvas = document.getElementById('bg-canvas');
    if (!canvas) return;

    try {
      const THREE = await import('three');
      const { EffectComposer } = await import('three/addons/postprocessing/EffectComposer.js');
      const { RenderPass } = await import('three/addons/postprocessing/RenderPass.js');
      const { UnrealBloomPass } = await import('three/addons/postprocessing/UnrealBloomPass.js');
      const { OutputPass } = await import('three/addons/postprocessing/OutputPass.js');

      const renderer = new THREE.WebGLRenderer({ canvas, alpha: false, antialias: true, powerPreference: 'high-performance' });
      renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
      renderer.toneMapping = THREE.ACESFilmicToneMapping;
      renderer.toneMappingExposure = 1.0;

      const scene = new THREE.Scene();
      scene.background = new THREE.Color(0x0a0a0f);

      const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 100);
      camera.position.set(0, 0, 4);

      const N = 2000;
      const pos = new Float32Array(N * 3);
      const colors = new Float32Array(N * 3);
      const sizes = new Float32Array(N);
      const v = new Float32Array(N);
      const u = new Float32Array(N);
      const c1 = new THREE.Color('#C1121F');
      const c2 = new THREE.Color('#F97316');
      const types = new Uint8Array(N);

      for (let i = 0; i < N; i++) {
        pos[i*3] = (Math.random()-0.5)*8;
        pos[i*3+1] = (Math.random()-0.5)*5;
        pos[i*3+2] = (Math.random()-0.5)*2;
        v[i] = -65 + Math.random()*10;
        u[i] = Math.random()*10;
        sizes[i] = 2 + Math.random()*3;
        const t = Math.floor(Math.random()*4);
        types[i] = t;
        if (t===1) sizes[i] *= 0.6;
        else if (t===2) sizes[i] *= 1.3;
      }

      const geo = new THREE.BufferGeometry();
      geo.setAttribute('position', new THREE.BufferAttribute(pos, 3));
      geo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
      geo.setAttribute('size', new THREE.BufferAttribute(sizes, 1));

      const mat = new THREE.PointsMaterial({
        size: 0.04, vertexColors: true, transparent: true, opacity: 0.9,
        blending: THREE.AdditiveBlending, depthWrite: false,
        sizeAttenuation: true
      });
      const points = new THREE.Points(geo, mat);
      scene.add(points);

      const composer = new EffectComposer(renderer);
      composer.addPass(new RenderPass(scene, camera));
      composer.addPass(new UnrealBloomPass(new THREE.Vector2(window.innerWidth, window.innerHeight), 0.4, 0.3, 0.85));
      composer.addPass(new OutputPass());

      const resize = () => {
        const w = window.innerWidth, h = window.innerHeight;
        camera.aspect = w/h;
        camera.updateProjectionMatrix();
        renderer.setSize(w, h);
        composer.setSize(w, h);
      };
      window.addEventListener('resize', resize);
      resize();

      let spikeAccum = 0;
      const mouse = new THREE.Vector3(0, 0, 0);
      canvas.addEventListener('click', (e) => {
        mouse.x = (e.clientX/window.innerWidth)*8-4;
        mouse.y = -(e.clientY/window.innerHeight)*5+2.5;
        spikeAccum += 3;
        this.sonifySpike(0.5);
      });

      const start = performance.now();
      const frame = () => {
        const t = (performance.now()-start)/1000;
        const params = [
          {a:0.02,b:0.2,c:-65,d:8},{a:0.1,b:0.2,c:-65,d:2},
          {a:0.02,b:0.2,c:-55,d:4},{a:0.025,b:0.2,c:-65,d:2}
        ];
        let spikesThisFrame = 0;
        for (let i=0; i<N; i++) {
          const p = params[types[i]];
          const dx=pos[i*3]-mouse.x, dy=pos[i*3+1]-mouse.y;
          const mouseI = Math.exp(-(dx*dx+dy*dy)*2)*12;
          const noise = Math.sin(i*12.9898+t*1.3)*0.5+1.5;
          const I = 5+noise+mouseI;
          for (let s=0;s<2;s++){
            v[i]+=0.5*(p.a*v[i]*v[i]+p.b*v[i]+140-u[i]+I);
            u[i]+=0.5*p.a*(p.b*v[i]-u[i]);
          }
          if (v[i]>=30){
            v[i]=p.c;u[i]+=p.d;
            spikesThisFrame++;
            colors[i*3]=1;colors[i*3+1]=0.45;colors[i*3+2]=0;
            pos[i*3]+=(Math.random()-0.5)*0.05;
            pos[i*3+1]+=(Math.random()-0.5)*0.05;
          }else{
            const blend=types[i]/3;
            colors[i*3]=c1.r+(c2.r-c1.r)*blend;
            colors[i*3+1]=c1.g+(c2.g-c1.g)*blend;
            colors[i*3+2]=c1.b+(c2.b-c1.b)*blend;
          }
        }
        spikeAccum+=spikesThisFrame;
        geo.attributes.color.needsUpdate=true;
        geo.attributes.position.needsUpdate=true;
        points.rotation.z=Math.sin(t*0.05)*0.02;
        composer.render();
        this.state._frameCount++;
        if(this.state._frameCount%30===0){
          this.state._spikeRate=spikeAccum/30;
          spikeAccum=0;
          const srEl=document.getElementById('spike-rate');
          if(srEl)srEl.textContent=this.state._spikeRate.toFixed(1);
        }
        requestAnimationFrame(frame);
      };
      frame();
    } catch(e) {
      this.renderFallback(canvas);
    }
  },

  renderFallback(canvas) {
    const ctx = canvas.getContext('2d');
    const resize = () => { canvas.width = window.innerWidth; canvas.height = window.innerHeight; };
    window.addEventListener('resize', resize);
    resize();
    const particles = Array.from({length:100}, () => ({
      x: Math.random()*canvas.width, y: Math.random()*canvas.height,
      vx: (Math.random()-0.5)*2, vy: (Math.random()-0.5)*2, r: Math.random()*2+1
    }));
    const frame = () => {
      ctx.fillStyle = 'rgba(18,18,18,0.05)';
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      for (const p of particles) {
        p.x+=p.vx; p.y+=p.vy;
        if(p.x<0||p.x>canvas.width)p.vx*=-1;
        if(p.y<0||p.y>canvas.height)p.vy*=-1;
        ctx.fillStyle='rgba(234,79,22,'+(0.2+p.r*0.1)+')';
        ctx.beginPath(); ctx.arc(p.x,p.y,p.r,0,Math.PI*2); ctx.fill();
      }
      requestAnimationFrame(frame);
    };
    frame();
  },

  // ═══ Audio Sonification ═══

  initAudio() {
    try { const C = window.AudioContext||window.webkitAudioContext; if(C) this.state._audioCtx=new C(); } catch(e){}
  },

  playTone(freq, dur, type='sine', vol=0.15) {
    if(!this.state._audioCtx) return;
    const ctx=this.state._audioCtx, o=ctx.createOscillator(), g=ctx.createGain();
    o.type=type; o.frequency.value=freq;
    g.gain.setValueAtTime(vol,ctx.currentTime);
    g.gain.exponentialRampToValueAtTime(0.001,ctx.currentTime+dur);
    o.connect(g); g.connect(ctx.destination); o.start(); o.stop(ctx.currentTime+dur);
  },

  sonifySpike(rate) {
    const pentatonic=[262,294,330,392,440,524,588,660,784,880];
    const freq=pentatonic[Math.floor(rate*pentatonic.length)%pentatonic.length];
    this.playTone(freq,0.08,'sine',Math.min(0.2,rate*0.5));
  },

  playConfirm(){this.playTone(523,0.1);setTimeout(()=>this.playTone(659,0.1),80);setTimeout(()=>this.playTone(784,0.15),160)},
  playCancel(){this.playTone(330,0.15,'sawtooth')},
  playSuccess(){this.playTone(784,0.08);setTimeout(()=>this.playTone(1047,0.15),80)},
  playError(){this.playTone(200,0.2,'sawtooth')},
  playOrder(){this.playTone(600,0.08);setTimeout(()=>this.playTone(800,0.08),60);setTimeout(()=>this.playTone(1000,0.12),120)},

  // ═══ Rendering ═══

  render() {
    document.getElementById('app').innerHTML = this.renderApp();
    this.renderMenu();
    this.renderOrders();
    this.renderCart();
  },

  renderApp() {
    const pages = ['menu','orders','analytics'];
    return `
    <nav class="navbar">
      <div class="navbar-logo gradient-text spring">dowiz</div>
      <div class="navbar-links">
        ${pages.map(p =>
          `<a href="#" class="${this.state.page===p?'active':''}" data-page="${p}">${p==='menu'?'Меню':p==='orders'?'Замовлення':'Аналітика'}</a>`
        ).join('')}
      </div>
      <button class="btn btn-ghost btn-sm desktop-only" onclick="App.toggleCart()">
        Кошик (${this.state.cart.reduce((s,i)=>s+i.qty,0)})
      </button>
    </nav>

    <main id="main-content">
      ${this.state.page==='menu'?this.pageMenu():''}
      ${this.state.page==='orders'?this.pageOrders():''}
      ${this.state.page==='analytics'?this.pageAnalytics():''}
    </main>

    <div class="cart-panel" id="cart-panel">
      <div class="cart-header"><h3>Кошик</h3><button class="btn btn-ghost btn-sm" onclick="App.toggleCart()">✕</button></div>
      <div class="cart-items" id="cart-items"></div>
      <div class="cart-total"><span>Разом</span><span id="cart-total">0 ALL</span></div>
      <div class="cart-actions"><button class="btn btn-primary w-full" onclick="App.checkout()">Замовити</button></div>
    </div>

    <footer><p>dowiz — децентралізований протокол доставки. ${this.state._stats.tests} тестів проходять.</p></footer>`;
  },

  pageMenu() {
    const cats=[...new Set(this.state._menu.map(i=>i.cat))];
    return `
    <section class="menu-section">
      <h2 class="section-title">Меню</h2>
      <p class="section-subtitle">Свіжі страви, локальні продукти, приготовані з любов\'ю.</p>
      <div class="cat-filters" id="cat-filters">
        <button class="btn btn-sm btn-primary" data-cat="all" onclick="App.filterMenu('all')">Усі</button>
        ${cats.map(c=>`<button class="btn btn-sm btn-ghost" data-cat="${c}" onclick="App.filterMenu('${c}')">${c}</button>`).join('')}
      </div>
      <div class="menu-grid" id="menu-grid"></div>
    </section>`;
  },

  renderMenu() {
    const items=this.state.filter==='all'?this.state._menu:this.state._menu.filter(i=>i.cat===this.state.filter);
    const grid=document.getElementById('menu-grid');
    if(!grid)return;
    grid.innerHTML=items.map(i=>`
      <div class="menu-item spring-fast">
        <div class="menu-img">${i.emoji}</div>
        <div class="menu-cat">${i.cat} \u00B7 ${i.prep}</div>
        <h4>${i.name}</h4>
        <p class="menu-desc">${i.desc}</p>
        <div class="menu-footer">
          <span class="menu-price">${i.price.toLocaleString()} ALL</span>
          <button class="btn btn-primary btn-sm" onclick="App.addToCart(${i.id})">Додати</button>
        </div>
      </div>`).join('');
  },

  filterMenu(cat) {
    this.state.filter=cat;
    this.renderMenu();
    document.querySelectorAll('#cat-filters .btn').forEach(b=>
      b.className=`btn btn-sm ${b.dataset.cat===cat?'btn-primary':'btn-ghost'}`
    );
  },

  pageOrders() {
    return `<section><h2 class="section-title">Замовлення</h2><p class="section-subtitle">Стежте за доставкою в реальному часі.</p><div class="orders-card" id="orders-card"></div></section>`;
  },

  renderOrders() {
    const el=document.getElementById('orders-card');
    if(!el)return;
    const colors={pending:'#D97706',confirmed:'#2563EB',preparing:'#F59E0B',ready:'#0D9488','in-delivery':'#3B82F6',delivered:'#059669'};
    const labels={pending:'Очікує',confirmed:'Підтверджено',preparing:'Готується',ready:'Готово','in-delivery':'В дорозі',delivered:'Доставлено'};
    el.innerHTML=this.state._orders.map(o=>`
      <div class="order-row spring-fast">
        <div><div class="order-id">#${o.id}</div><div class="order-meta">${o.items} позицій \u00B7 ${o.total.toLocaleString()} ALL \u00B7 ${o.time}</div></div>
        <div class="order-status">
          <span class="status-dot" style="background:${colors[o.status]};animation:${o.status!=='delivered'?'pulse 2s infinite':'none'}"></span>
          <span class="order-badge ${o.status}">${labels[o.status]||o.status}</span>
        </div>
      </div>`).join('');
  },

  pageAnalytics() {
    const s=this.state._stats;
    return `
    <section>
      <h2 class="section-title">Аналітика мережі</h2>
      <p class="section-subtitle">Метрики в реальному часі з mesh-мережі dowiz.</p>
      <div class="stats-grid">
        <div class="stat-card spring"><div class="stat-number">${s.orders}</div><div class="stat-label">Замовлень доставлено</div></div>
        <div class="stat-card spring"><div class="stat-number">${s.nodes}</div><div class="stat-label">Вузлів мережі</div></div>
        <div class="stat-card spring"><div class="stat-number">${s.uptime}</div><div class="stat-label">Аптайм мережі</div></div>
        <div class="stat-card spring"><div class="stat-number">${s.tests}</div><div class="stat-label">Тестів проходять</div></div>
      </div>
      <div class="test-grid">
        ${[{label:'Kernel',value:'1,749',color:'var(--brand-primary)'},
           {label:'Engine',value:'130',color:'var(--brand-primary-hover)'},
           {label:'Node',value:'3',color:'var(--color-success)'},
           {label:'Bebop',value:'68',color:'var(--color-info)'}
        ].map(b=>`<div class="test-card"><div class="test-value" style="color:${b.color}">${b.value}</div><div class="test-label">${b.label}</div></div>`).join('')}
      </div>
    </section>`;
  },

  // ═══ Cart ═══

  addToCart(id) {
    const item=this.state._menu.find(i=>i.id===id);
    if(!item)return;
    const existing=this.state.cart.find(i=>i.id===id);
    if(existing)existing.qty++;
    else this.state.cart.push({...item,qty:1});
    this.playConfirm();
    this.renderCart();
    this.toggleCart(true);
  },

  removeFromCart(id) {
    this.state.cart=this.state.cart.filter(i=>i.id!==id);
    this.playCancel();
    this.renderCart();
  },

  renderCart() {
    const count=this.state.cart.reduce((s,i)=>s+i.qty,0);
    const total=this.state.cart.reduce((s,i)=>s+i.price*i.qty,0);
    const el=document.getElementById('cart-items');
    const tEl=document.getElementById('cart-total');
    if(!el)return;
    el.innerHTML=count===0
      ?'<p style="text-align:center;color:var(--brand-text-muted);padding:48px 0">Кошик порожній</p>'
      :this.state.cart.map(i=>
        `<div class="cart-item spring-fast">
          <div><strong>${i.name}</strong><span class="text-muted"> \u00D7${i.qty}</span></div>
          <div style="display:flex;align-items:center;gap:8px">
            <span>${(i.price*i.qty).toLocaleString()} ALL</span>
            <button class="btn-remove" onclick="App.removeFromCart(${i.id})">\u2715</button>
          </div>
        </div>`).join('');
    if(tEl)tEl.textContent=`${total.toLocaleString()} ALL`;
  },

  toggleCart(open) {
    const panel=document.getElementById('cart-panel');
    if(open!==undefined)this.state._cartOpen=open;
    else this.state._cartOpen=!this.state._cartOpen;
    panel?.classList.toggle('open',this.state._cartOpen);
  },

  checkout() {
    if(this.state.cart.length===0){this.playError();return}
    const total=this.state.cart.reduce((s,i)=>s+i.price*i.qty,0);
    this.playOrder();
    this.state._orders.unshift({
      id:1000+Date.now()%10000,status:'pending',
      items:this.state.cart.length,total,time:'щойно'
    });
    this.state.cart=[];
    this.renderCart();
    this.renderOrders();
    this.toggleCart(false);
    this.navigate('orders');
  },

  // ═══ Navigation & Theme ═══

  navigate(page) {
    this.state.page=page;
    this.state._cartOpen=false;
    document.getElementById('cart-panel')?.classList.remove('open');
    this.render();
    document.querySelectorAll('.navbar-links a').forEach(a=>
      a.classList.toggle('active',a.dataset.page===page)
    );
    window.scrollTo({top:0,behavior:'smooth'});
  },

  setTheme(t) {
    this.state.theme=t;
    document.documentElement.dataset.theme=t;
    localStorage.setItem('dowiz-theme',t);
    document.querySelectorAll('.theme-dot').forEach(d=>
      d.classList.toggle('active',d.dataset.theme===t)
    );
  },

  bindEvents() {
    document.addEventListener('click',(e)=>{
      const link=e.target.closest('[data-page]');
      if(link){e.preventDefault();this.navigate(link.dataset.page)}
    });

    const observer=new IntersectionObserver((entries)=>{
      entries.forEach(e=>{if(e.isIntersecting)e.target.classList.add('visible')});
    },{threshold:0.1});
    document.querySelectorAll('.stagger').forEach(el=>observer.observe(el));

    document.addEventListener('keydown',(e)=>{
      if(e.key==='1')this.navigate('menu');
      else if(e.key==='2')this.navigate('orders');
      else if(e.key==='3')this.navigate('analytics');
      else if(e.key==='c')this.toggleCart();
      else if(e.key==='Escape')this.toggleCart(false);
    });
  }
};

document.addEventListener('DOMContentLoaded',()=>App.init());
window.App=App;
