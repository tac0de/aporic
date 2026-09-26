async (page) => {
  const arm = page.url().includes(':8781') ? 'X' : 'Y';
  const base = `/private/tmp/aporic-ab-blind/${arm}`;
  const errors = [], failed = [], requests = [];
  page.on('console', m => { if(m.type() === 'error') errors.push(m.text()); });
  page.on('pageerror', e => errors.push(e.message));
  page.on('requestfailed', r => failed.push({url:r.url(), error:r.failure()}));
  page.on('request', r => requests.push(r.url()));
  const output = {arm, views:[], errors, failed, requests};
  const layout = async () => page.evaluate(() => {
    const rect = e => { const r=e.getBoundingClientRect();return {left:r.left,top:r.top,right:r.right,bottom:r.bottom,width:r.width,height:r.height}; };
    const root=document.documentElement;
    const controls=[...document.querySelectorAll('button,a,input')].filter(e=>e.getClientRects().length && getComputedStyle(e).visibility!=='hidden');
    return {viewport:{width:innerWidth,height:innerHeight},scrollWidth:root.scrollWidth,clientWidth:root.clientWidth,overflow:controls.filter(e=>{const r=e.getBoundingClientRect();return r.left<-.5||r.right>innerWidth+.5}).map(e=>({text:e.textContent,rect:rect(e)})),dialog:document.querySelector('dialog[open]')?rect(document.querySelector('dialog[open]')):null};
  });
  const visibleTitles = () => page.locator('.proposal-card:visible h3').allTextContents();
  for(const [width,height] of [[1440,900],[390,844]]) {
    await page.setViewportSize({width,height});
    await page.emulateMedia({reducedMotion:'no-preference'});
    await page.reload();
    const view={size:`${width}x${height}`};
    view.initial={titles:await visibleTitles(),progress:await page.getByRole('progressbar').getAttribute('aria-valuenow'),layout:await layout(),aria:await page.locator('main').ariaSnapshot()};
    await page.screenshot({path:`${base}-${width}-initial.png`,fullPage:true});
    view.dialogs=[];
    for(let i=0;i<3;i++) {
      const opener=page.locator('.proposal-card button').nth(i);
      await opener.focus();
      await page.keyboard.press('Enter');
      const dialog=page.getByRole('dialog');
      const d={index:i,title:await page.locator('#dialog-title').textContent(),snapshot:await dialog.ariaSnapshot(),layout:await layout(),disabled:[]};
      for(let count=0;count<=3;count++) {
        d.disabled.push(await page.locator('#mark-reviewed').isDisabled());
        if(count<3) await dialog.locator('label').nth(count).click();
      }
      await dialog.locator('label').nth(1).click();
      d.disabledAfterUncheck=await page.locator('#mark-reviewed').isDisabled();
      if(i===0) await page.screenshot({path:`${base}-${width}-dialog.png`});
      await page.keyboard.press('Escape');
      await page.waitForFunction(()=>!document.querySelector('dialog').open);
      await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
      d.restoredFocus=await opener.evaluate(e=>document.activeElement===e);
      view.dialogs.push(d);
    }
    const first=page.locator('.proposal-card button').first();
    await first.click();
    const dialog=page.getByRole('dialog');
    view.keyboard=[];
    for(let i=0;i<6;i++) {await page.keyboard.press('Tab');view.keyboard.push(await page.evaluate(()=>({tag:document.activeElement.tagName,text:document.activeElement.textContent,type:document.activeElement.getAttribute('type'),insideDialog:!!document.activeElement.closest('dialog')})));}
    for(let i=0;i<3;i++) await dialog.locator('label').nth(i).click();
    await page.locator('#mark-reviewed').click();
    await page.waitForFunction(()=>!document.querySelector('dialog').open);
      await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    view.completed={progress:await page.getByRole('progressbar').getAttribute('aria-valuenow'),firstStatus:await page.locator('.proposal-card .status').first().textContent(),focus:await page.evaluate(()=>({tag:document.activeElement.tagName,text:document.activeElement.textContent,aria:document.activeElement.getAttribute('aria-label')}))};
    view.filters={};
    for(const filter of ['All','To review','Reviewed']) {
      await page.getByRole('button',{name:new RegExp(`^${filter} `)}).click();
      view.filters[filter]=await visibleTitles();
    }
    await page.getByRole('button',{name:/^All /}).click();
    await page.locator('.proposal-card button').first().click();
    view.reviewedDialog={snapshot:await page.getByRole('dialog').ariaSnapshot(),markVisible:await page.locator('#mark-reviewed').isVisible(),markHiddenAttribute:await page.locator('#mark-reviewed').getAttribute('hidden')};
    await page.keyboard.press('Escape');
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    await page.screenshot({path:`${base}-${width}-completed.png`,fullPage:true});
    await page.emulateMedia({reducedMotion:'reduce'});
    view.reducedMotion=await page.evaluate(()=>({matches:matchMedia('(prefers-reduced-motion: reduce)').matches,transition:getComputedStyle(document.querySelector('#progress-fill')).transitionDuration,animation:getComputedStyle(document.querySelector('.proposal-card')).animationDuration,scroll:getComputedStyle(document.documentElement).scrollBehavior}));
    output.views.push(view);
  }
  return output;
}
