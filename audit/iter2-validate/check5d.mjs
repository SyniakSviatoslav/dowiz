import { chromium } from '@playwright/test';
const BASE='https://dowiz-staging.fly.dev', OWNER=process.env.OWNER_TOKEN, OUT='audit/iter2-validate';
const b=await chromium.launch(); const c=await b.newContext({viewport:{width:1280,height:900}});
await c.addInitScript(t=>localStorage.setItem('dos_access_token',t),OWNER);
const p=await c.newPage();
const errs=[]; p.on('console',m=>{if(m.type()==='error')errs.push(m.text())}); p.on('pageerror',e=>errs.push('pe '+e.message));
await p.goto(BASE+'/admin/menu',{waitUntil:'networkidle',timeout:60000}); await p.waitForTimeout(2500);

// Click "Shto Kategori" (add category) — opens modal; we will NOT submit (read-only).
const addCat = p.locator('button[aria-label="Shto Kategori"]');
let dlg=0, trigger='Shto Kategori';
if(await addCat.count()){ await addCat.first().click().catch(()=>{}); await p.waitForTimeout(1000); dlg=await p.locator('[role="dialog"]').count(); }

// fallback: open product editor via pencil (the LAST icon button in a card is delete, the one before edit)
if(!dlg){
  trigger='product-pencil';
  const did = await p.evaluate(()=>{
    let card=null;
    for(const e of document.querySelectorAll('div')){const tx=e.textContent||'';if(/Cola/.test(tx)&&/200 ALL/.test(tx)&&tx.length<300){card=e;break;}}
    if(!card)return 'no-card';
    const bs=[...card.querySelectorAll('button')];
    // skip the toggle switch (rounded-full w-8). pick a button containing an svg path that's the pencil
    const action=bs.filter(x=>x.querySelector('svg'));
    if(action.length>=2){action[action.length-2].click();return 'clicked-pencil';}
    return 'no-pencil';
  });
  await p.waitForTimeout(1000);
  dlg=await p.locator('[role="dialog"]').count();
}

await p.screenshot({path:`${OUT}/05b-admin-modal.png`});
let closeInfo='none', animInfo='n/a';
if(dlg){
  const dialog=p.locator('[role="dialog"]').first();
  // record any transition/animation classes
  animInfo = await p.evaluate(()=>{
    const d=document.querySelector('[role="dialog"]'); if(!d)return 'none';
    const cs=getComputedStyle(d);
    return `anim=${cs.animationName} transDur=${cs.transitionDuration} cls=${(d.className||'').toString().slice(0,70)}`;
  });
  const close=p.locator('[role="dialog"] button').first();
  if(await close.count()){
    await close.focus().catch(()=>{});
    closeInfo=await p.evaluate(()=>{const el=document.activeElement;if(!el)return'na';const cs=getComputedStyle(el);return`tag=${el.tagName} aria=${el.getAttribute('aria-label')} outline=${cs.outlineWidth}/${cs.outlineStyle} ring=${(cs.boxShadow||'').slice(0,50)}`;});
    await p.screenshot({path:`${OUT}/05c-modal-close-focus.png`});
  }
}
console.log(JSON.stringify({trigger,dlg,animInfo,closeInfo,errs:errs.slice(0,8)},null,2));
await b.close();
