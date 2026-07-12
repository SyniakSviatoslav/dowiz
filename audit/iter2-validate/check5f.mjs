import { chromium } from '@playwright/test';
const BASE='https://dowiz-staging.fly.dev', OWNER=process.env.OWNER_TOKEN, OUT='audit/iter2-validate';
const b=await chromium.launch(); const c=await b.newContext({viewport:{width:1280,height:900}});
await c.addInitScript(t=>localStorage.setItem('dos_access_token',t),OWNER);
const p=await c.newPage();
const errs=[]; p.on('console',m=>{if(m.type()==='error')errs.push(m.text())}); p.on('pageerror',e=>errs.push('pe '+e.message));
await p.goto(BASE+'/admin/menu',{waitUntil:'networkidle',timeout:60000}); await p.waitForTimeout(2500);
// pencil is at approx (516,481) per earlier screenshot — click the edit icon of the Cola card
await p.mouse.click(516,481);
await p.waitForTimeout(1500);
const modal = await p.evaluate(()=>{
  let d=document.querySelector('[role="dialog"]');
  if(!d){const fx=[...document.querySelectorAll('*')].filter(e=>{const cs=getComputedStyle(e);return cs.position==='fixed'&&e.offsetHeight>300&&e.offsetWidth>300&&parseInt(cs.zIndex||0)>=20;});d=fx[0];}
  if(!d)return {present:false};
  const cs=getComputedStyle(d);
  return {present:true,role:d.getAttribute('role'),anim:cs.animationName,transDur:cs.transitionDuration,cls:(d.className||'').toString().slice(0,90),text:(d.innerText||'').slice(0,80).replace(/\n/g,' ')};
});
await p.screenshot({path:`${OUT}/05b-admin-modal.png`});
let closeInfo='none';
if(modal.present){
  closeInfo=await p.evaluate(()=>{
    const d=document.querySelector('[role="dialog"]')||[...document.querySelectorAll('*')].find(e=>getComputedStyle(e).position==='fixed'&&e.offsetHeight>300);
    const btn=d&&d.querySelector('button[aria-label],button'); if(!btn)return'no-btn'; btn.focus();
    const el=document.activeElement;const cs=getComputedStyle(el);
    return `tag=${el.tagName} aria=${el.getAttribute('aria-label')} outline=${cs.outlineWidth}/${cs.outlineStyle} ring=${(cs.boxShadow||'').slice(0,60)}`;
  });
  await p.screenshot({path:`${OUT}/05c-modal-close-focus.png`});
  // close WITHOUT saving — press Escape
  await p.keyboard.press('Escape'); await p.waitForTimeout(500);
}
console.log(JSON.stringify({modal,closeInfo,errs:errs.slice(0,8)},null,2));
await b.close();
