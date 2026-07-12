import { chromium } from '@playwright/test';
const BASE='https://dowiz-staging.fly.dev', OWNER=process.env.OWNER_TOKEN, OUT='audit/iter2-validate';
const b=await chromium.launch(); const c=await b.newContext({viewport:{width:1280,height:900}});
await c.addInitScript(t=>localStorage.setItem('dos_access_token',t),OWNER);
const p=await c.newPage();
await p.goto(BASE+'/admin/menu',{waitUntil:'networkidle',timeout:60000}); await p.waitForTimeout(2500);
// dump all buttons w/ aria-label/title/text near top
const btns = await p.evaluate(()=>{
  return [...document.querySelectorAll('button')].slice(0,40).map(b=>({
    t:(b.innerText||'').trim().slice(0,20), al:b.getAttribute('aria-label'), ti:b.getAttribute('title')
  }));
});
console.log('BUTTONS:', JSON.stringify(btns));
// click first pencil: find svg buttons inside the product grid
const before = await p.locator('[role="dialog"]').count();
// click the first edit-looking icon button after the category chips. Heuristic: buttons with no text + svg
const iconBtns = p.locator('button:not(:has-text("")) ');
// Instead, click the pencil in Cola card by text proximity using evaluate
const clicked = await p.evaluate(()=>{
  const cards=[...document.querySelectorAll('*')].filter(e=>e.children.length && /Cola/.test(e.textContent||'') && (e.textContent||'').length<400);
  // find smallest card containing Cola price
  let card=null;
  for(const e of document.querySelectorAll('div')){ const tx=e.textContent||''; if(/Cola/.test(tx)&&/200 ALL/.test(tx)&&/disponuesh/i.test(tx)&&tx.length<300){card=e;break;} }
  if(!card) return 'no-card';
  const bs=[...card.querySelectorAll('button')];
  if(!bs.length) return 'no-btn-in-card';
  // pencil is usually first of the two action buttons (edit, delete)
  bs[0].click();
  return 'clicked '+bs.length+' btns, firstHTML='+bs[0].outerHTML.slice(0,80);
});
console.log('CLICK:', clicked);
await p.waitForTimeout(1200);
const after = await p.locator('[role="dialog"]').count();
// also check for any fixed overlay
const overlay = await p.evaluate(()=>{
  const fx=[...document.querySelectorAll('*')].filter(e=>{const cs=getComputedStyle(e);return cs.position==='fixed'&&parseInt(cs.zIndex||0)>=30&&e.offsetHeight>200;});
  return fx.slice(0,3).map(e=>({tag:e.tagName,cls:(e.className||'').toString().slice(0,60),role:e.getAttribute('role')}));
});
console.log('dialogBefore='+before+' dialogAfter='+after);
console.log('FIXED-OVERLAYS:', JSON.stringify(overlay));
await p.screenshot({path:`${OUT}/05b-admin-modal.png`});
await b.close();
