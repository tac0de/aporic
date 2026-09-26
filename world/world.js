import { GOVERNMENT_SNAPSHOT as DATA } from './government-data.js';

const canvas = document.querySelector('#world');
const scene = document.querySelector('#scene');
const labelsHost = document.querySelector('#labels');
const districtsHost = document.querySelector('#districts');
const peopleHost = document.querySelector('#people');
const detail = document.querySelector('#detail');
const $ = (selector) => document.querySelector(selector);
const gl = canvas.getContext('webgl', { antialias: true, alpha: true });
const palette = {
  ground: '#527a75', path: '#d4c7a7', plaza: '#e5d5b7', plazaEdge: '#b9a685',
  product: '#d28d5b', memory: '#6c9bb0', operations: '#78a486', inspector: '#a889ae',
  stone: '#ded4ba', cream: '#f5ead0', dark: '#284a57', gold: '#efc37f', leaf: '#397068',
};
const sites = [
  { id: 'product.experiment', title: '제품실험부', x: -17, z: -10, color: palette.product, eyebrow: 'MINISTRY 01', type: 'office' },
  { id: 'memory.information', title: '기억정보부', x: 17, z: -10, color: palette.memory, eyebrow: 'MINISTRY 02', type: 'office' },
  { id: 'execution.operations', title: '실행운영부', x: -17, z: 13, color: palette.operations, eyebrow: 'MINISTRY 03', type: 'office' },
  { id: 'oversight.inspector', title: '독립 감사실', x: 17, z: 13, color: palette.inspector, eyebrow: 'INDEPENDENT', type: 'district' },
  { id: 'executive.prime_minister', title: '국무총리', x: 0, z: -19, color: palette.gold, eyebrow: 'EXECUTIVE', type: 'person' },
];
const peoplePositions = {
  'executive.prime_minister': [0, -18],
  'product.experiment.minister': [-17, -5.5],
  'product.design.lead': [-23, -8],
  'product.frontend.lead': [-11.5, -8],
  'product.backend.lead': [-22.5, -13],
  'product.game.lead': [-11.5, -13],
  'memory.information.minister': [17, -5.5],
  'execution.operations.minister': [-17, 17.5],
  'oversight.inspector': [17, 18],
};
const personColor = (person) => person.officeId ? DATA.offices.find(o => o.id === person.officeId).color : person.independent ? palette.inspector : palette.gold;
const personById = Object.fromEntries(DATA.people.map(person => [person.id, person]));
const officeById = Object.fromEntries(DATA.offices.map(office => [office.id, office]));
$('#status-ministries').textContent=DATA.status.activeMinistries;
$('#status-people').textContent=DATA.status.activeOfficeholders;
$('#status-cells').textContent=DATA.status.activeProductCells;
$('#status-tasks').textContent=`진행 표기 ${DATA.status.taskRegistry.leased} · 대기 ${DATA.status.taskRegistry.queued} · 완료 ${DATA.status.taskRegistry.completed}`;
$('#status-time').textContent=`${DATA.status.capturedAt} · 자문 기록 기준`;
const colorRgb = (hex) => [1, 3, 5].map(i => parseInt(hex.slice(i, i + 2), 16) / 255);
const clamp = (value, min, max) => Math.max(min, Math.min(max, value));

let player = { x: 0, z: 8 };
let yaw = 0.45;
let selected = null;
let destination = null;
let overview = false;
let lastTime = performance.now();
let viewProjection;
const keys = new Set();
const labels = [];

function makeMesh() { return { vertices: [], indices: [] }; }
function quad(mesh, points, color, normal) {
  const start = mesh.vertices.length / 9;
  const rgb = colorRgb(color);
  for (const p of points) mesh.vertices.push(...p, ...rgb, ...normal);
  mesh.indices.push(start, start + 1, start + 2, start, start + 2, start + 3);
}
function box(mesh, x, y, z, w, h, d, color) {
  const a = x - w / 2, b = x + w / 2, c = y, e = y + h, f = z - d / 2, g = z + d / 2;
  quad(mesh, [[a,e,g],[b,e,g],[b,e,f],[a,e,f]], color, [0,1,0]);
  quad(mesh, [[a,c,g],[a,e,g],[a,e,f],[a,c,f]], color, [-1,0,0]);
  quad(mesh, [[b,c,f],[b,e,f],[b,e,g],[b,c,g]], color, [1,0,0]);
  quad(mesh, [[a,c,g],[b,c,g],[b,e,g],[a,e,g]], color, [0,0,1]);
  quad(mesh, [[b,c,f],[a,c,f],[a,e,f],[b,e,f]], color, [0,0,-1]);
}
function cylinder(mesh, x, y, z, radiusTop, radiusBottom, height, color, sides = 12) {
  const rgb = colorRgb(color), start = mesh.vertices.length / 9;
  for (let i = 0; i <= sides; i++) {
    const angle = i / sides * Math.PI * 2, sx = Math.cos(angle), sz = Math.sin(angle);
    mesh.vertices.push(x + sx * radiusBottom, y, z + sz * radiusBottom, ...rgb, sx, 0.25, sz);
    mesh.vertices.push(x + sx * radiusTop, y + height, z + sz * radiusTop, ...rgb, sx, 0.25, sz);
    if (i < sides) {
      const p = start + i * 2;
      mesh.indices.push(p, p+1, p+2, p+1, p+3, p+2);
    }
  }
  const top = mesh.vertices.length / 9;
  mesh.vertices.push(x, y + height, z, ...rgb, 0, 1, 0);
  for (let i=0;i<sides;i++) mesh.indices.push(top, start+i*2+1, start+(i+1)*2+1);
}
function sphere(mesh, x, y, z, radius, color) {
  const rgb = colorRgb(color), segments = 10, rings = 7, start = mesh.vertices.length / 9;
  for(let j=0;j<=rings;j++) {
    const v=j/rings*Math.PI;
    for(let i=0;i<=segments;i++) {
      const a=i/segments*Math.PI*2, nx=Math.sin(v)*Math.cos(a), ny=Math.cos(v), nz=Math.sin(v)*Math.sin(a);
      mesh.vertices.push(x+nx*radius,y+ny*radius,z+nz*radius,...rgb,nx,ny,nz);
    }
  }
  for(let j=0;j<rings;j++) for(let i=0;i<segments;i++) {
    const p=start+j*(segments+1)+i;
    mesh.indices.push(p,p+segments+1,p+1,p+1,p+segments+1,p+segments+2);
  }
}
function walkway(mesh, x1, z1, x2, z2, width=2.7) {
  const dx=x2-x1,dz=z2-z1,length=Math.hypot(dx,dz),nx=-dz/length*width/2,nz=dx/length*width/2;
  quad(mesh,[[x1+nx,.055,z1+nz],[x2+nx,.055,z2+nz],[x2-nx,.055,z2-nz],[x1-nx,.055,z1-nz]],palette.path,[0,1,0]);
}
function building(mesh, site, index) {
  const {x,z,color}=site;
  const w=index===3?7.2:8.6,d=index===3?6.5:7.5,h=index===3?3.5:4.3;
  box(mesh,x,-.02,z,w+2,.34,d+2,palette.stone);
  box(mesh,x,.2,z,w,h,d,palette.cream);
  box(mesh,x-w/2-.22,.2,z,.45,h+.25,d+.3,color);
  box(mesh,x+w/2+.22,.2,z,.45,h+.25,d+.3,color);
  box(mesh,x,.2,z-d/2-.22,w+.7,h+.25,.48,color);
  box(mesh,x,.2,z+d/2+.22,w+.7,h+.25,.48,color);
  box(mesh,x,.2+h,z,w+1.2,.45,d+1.2,color);
  box(mesh,x,.68+h,z,w+.4,.14,d+.4,palette.stone);
  box(mesh,x,.3,z+d/2+.31,1.45,2.5,.08,palette.dark);
  box(mesh,x,.4,z+d/2+.38,.08,1.8,.08,palette.gold);
  for(const side of [-1,1]) for(const offset of [-2.4,2.4]) {
    box(mesh,x+offset,1.25,z+side*(d/2+.32),1.2,1.45,.06,palette.memory);
    box(mesh,x+offset,1.25,z+side*(d/2+.38),.08,1.5,.06,palette.stone);
  }
  for(const dx of [-w/2+1,w/2-1]) {
    cylinder(mesh,x+dx,.45,z+d/2+1.25,.22,.28,3.25,palette.stone);
    box(mesh,x+dx,3.62,z+d/2+1.25,.7,.24,.7,color);
  }
  box(mesh,x,3.62,z+d/2+1.24,w-1,.3,.7,palette.stone);
  cylinder(mesh,x,.72+h,z,1.3,1.5,.64,color,16);
  cylinder(mesh,x,1.38+h,z,.16,.65,1.2,palette.gold,12);
}
function figure(mesh,x,z,color,large=false) {
  const scale=large?1.17:1;
  cylinder(mesh,x,.05,z,.47*scale,.57*scale,.16,palette.dark);
  cylinder(mesh,x,.22,z,.29*scale,.38*scale,1.16*scale,color);
  sphere(mesh,x,1.68*scale,z,.38*scale,'#f4d9b6');
  cylinder(mesh,x,1.89*scale,z,.25*scale,.37*scale,.19,palette.dark);
}
function tree(mesh,x,z,s=1) {
  cylinder(mesh,x,.05,z,.16*s,.18*s,1.8*s,'#806d55');
  cylinder(mesh,x,1.4*s,z,.16*s,1.15*s,2.9*s,palette.leaf,9);
  cylinder(mesh,x,3.35*s,z,0,.8*s,1.3*s,'#4e8977',9);
}
function buildWorld() {
  const mesh=makeMesh();
  box(mesh,0,-.28,0,80,.3,80,palette.ground);
  for(let i=-36;i<=36;i+=6) {
    box(mesh,i,.01,-39,.018,.015,78,'#5d867d');
    box(mesh,-39,.01,i,78,.015,.018,'#5d867d');
  }
  for(const site of sites.slice(0,4)) walkway(mesh,0,0,site.x,site.z);
  walkway(mesh,0,0,0,-19,2.8);
  cylinder(mesh,0,.035,0,6.2,6.2,.12,palette.plazaEdge,40);
  cylinder(mesh,0,.15,0,5.7,5.7,.09,palette.plaza,40);
  cylinder(mesh,0,.25,0,2.5,2.5,.19,palette.stone,32);
  cylinder(mesh,0,.43,0,1.95,1.95,.16,palette.memory,32);
  cylinder(mesh,0,.62,0,.76,.88,.7,palette.stone,20);
  sphere(mesh,0,1.65,0,.74,palette.gold);
  for(let i=0;i<4;i++) {
    const a=i*Math.PI/2+.72;
    cylinder(mesh,Math.cos(a)*4.5,.25,Math.sin(a)*4.5,.27,.32,.17,palette.gold,10);
  }
  sites.slice(0,4).forEach((s,i)=>building(mesh,s,i));
  box(mesh,0,.03,-20,5,.34,5,palette.stone);
  cylinder(mesh,0,.4,-20,1.65,1.85,.44,palette.dark,16);
  cylinder(mesh,0,.84,-20,.4,.72,1.3,palette.gold,12);
  for(const person of DATA.people) {
    const [x,z]=peoplePositions[person.id]; figure(mesh,x,z,personColor(person),person.kind==='prime-minister');
  }
  const trees=[[-32,-24],[-26,-22],[-8,-28],[8,-28],[27,-23],[33,-17],[-33,0],[32,0],[-33,25],[-26,29],[-7,27],[7,27],[27,29],[33,25],[-6,16],[7,15]];
  trees.forEach(([x,z],i)=>tree(mesh,x,z,.8+(i%3)*.12));
  return mesh;
}

function shader(type,source) {
  const result=gl.createShader(type); gl.shaderSource(result,source); gl.compileShader(result);
  if(!gl.getShaderParameter(result,gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(result));
  return result;
}
const vertexSource=`attribute vec3 position; attribute vec3 color; attribute vec3 normal;
uniform mat4 viewProjection; uniform vec3 translation; varying vec3 vColor; varying float vFog;
void main(){ vec4 clip=viewProjection*vec4(position+translation,1.0); gl_Position=clip;
float light=max(dot(normalize(normal),normalize(vec3(-0.45,0.88,0.56))),0.0);
vColor=color*(0.69+light*0.36); vFog=clamp((clip.w-24.0)/100.0,0.0,0.45); }`;
const fragmentSource=`precision mediump float; varying vec3 vColor; varying float vFog;
void main(){ gl_FragColor=vec4(mix(vColor,vec3(0.31,0.47,0.51),vFog),1.0); }`;
let program, attributes, uniform, translationUniform;
if(gl) {
  program=gl.createProgram(); gl.attachShader(program,shader(gl.VERTEX_SHADER,vertexSource)); gl.attachShader(program,shader(gl.FRAGMENT_SHADER,fragmentSource)); gl.linkProgram(program);
  if(!gl.getProgramParameter(program,gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program));
  attributes=['position','color','normal'].map(name=>gl.getAttribLocation(program,name));
  uniform=gl.getUniformLocation(program,'viewProjection');
  translationUniform=gl.getUniformLocation(program,'translation');
  gl.enable(gl.DEPTH_TEST); gl.enable(gl.CULL_FACE); gl.cullFace(gl.BACK);
} else {
  scene.insertAdjacentHTML('beforeend','<p class="webgl-error">이 브라우저에서는 3D 렌더링을 사용할 수 없습니다. WebGL 지원 브라우저에서 열어주세요.</p>');
}
function upload(mesh) {
  const vertex=gl.createBuffer(), index=gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER,vertex); gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(mesh.vertices),gl.STATIC_DRAW);
  gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,index); gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,new Uint16Array(mesh.indices),gl.STATIC_DRAW);
  return {vertex,index,count:mesh.indices.length};
}
const staticMesh=gl?upload(buildWorld()):null;
const avatarMesh=gl?upload((()=>{const mesh=makeMesh();figure(mesh,0,0,palette.gold,true);return mesh;})()):null;
function draw(mesh) {
  gl.bindBuffer(gl.ARRAY_BUFFER,mesh.vertex); gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,mesh.index);
  attributes.forEach((attribute,i)=>{gl.enableVertexAttribArray(attribute);gl.vertexAttribPointer(attribute,3,gl.FLOAT,false,36,i*12);});
  gl.drawElements(gl.TRIANGLES,mesh.count,gl.UNSIGNED_SHORT,0);
}
function multiply(a,b) {
  const out=new Float32Array(16);
  for(let c=0;c<4;c++) for(let r=0;r<4;r++) for(let k=0;k<4;k++) out[c*4+r]+=a[k*4+r]*b[c*4+k];
  return out;
}
function perspective(fov,aspect,near,far) {
  const f=1/Math.tan(fov/2),out=new Float32Array(16);
  out[0]=f/aspect;out[5]=f;out[10]=(far+near)/(near-far);out[11]=-1;out[14]=2*far*near/(near-far);return out;
}
function lookAt(eye,target) {
  let z=[eye[0]-target[0],eye[1]-target[1],eye[2]-target[2]];
  const zn=Math.hypot(...z);z=z.map(v=>v/zn);
  let x=[z[2],0,-z[0]],xn=Math.hypot(...x);x=x.map(v=>v/xn);
  const y=[z[1]*x[2]-z[2]*x[1],z[2]*x[0]-z[0]*x[2],z[0]*x[1]-z[1]*x[0]];
  return new Float32Array([x[0],y[0],z[0],0,x[1],y[1],z[1],0,x[2],y[2],z[2],0,-x[0]*eye[0]-x[1]*eye[1]-x[2]*eye[2],-y[0]*eye[0]-y[1]*eye[1]-y[2]*eye[2],-z[0]*eye[0]-z[1]*eye[1]-z[2]*eye[2],1]);
}
function project(x,y,z) {
  const m=viewProjection;
  const px=m[0]*x+m[4]*y+m[8]*z+m[12],py=m[1]*x+m[5]*y+m[9]*z+m[13],pw=m[3]*x+m[7]*y+m[11]*z+m[15];
  if(pw<=0) return null;
  return {x:(px/pw*.5+.5)*scene.clientWidth,y:(1-(py/pw*.5+.5))*scene.clientHeight,visible:Math.abs(px/pw)<1.15&&Math.abs(py/pw)<1.1};
}
function render(now) {
  const dt=Math.min((now-lastTime)/1000,.05);lastTime=now;
  const forward=(keys.has('w')||keys.has('arrowup')?1:0)-(keys.has('s')||keys.has('arrowdown')?1:0);
  const side=(keys.has('d')||keys.has('arrowright')?1:0)-(keys.has('a')||keys.has('arrowleft')?1:0);
  if(forward||side) {
    destination=null;
    const norm=Math.hypot(forward,side),f=forward/norm,s=side/norm,speed=10*dt;
    player.x=clamp(player.x+(-Math.sin(yaw)*f+Math.cos(yaw)*s)*speed,-32,32);
    player.z=clamp(player.z+(-Math.cos(yaw)*f-Math.sin(yaw)*s)*speed,-30,30);
  } else if(destination) {
    const dx=destination.x-player.x,dz=destination.z-player.z,dist=Math.hypot(dx,dz);
    if(dist<.18)destination=null;
    else {const step=Math.min(dist,10*dt);player.x+=dx/dist*step;player.z+=dz/dist*step;}
  }
  if(gl) {
    const dpr=Math.min(devicePixelRatio||1,2),w=Math.round(scene.clientWidth*dpr),h=Math.round(scene.clientHeight*dpr);
    if(canvas.width!==w||canvas.height!==h){canvas.width=w;canvas.height=h;gl.viewport(0,0,w,h);}
    const distance=overview?(scene.clientWidth<680?63:52):(scene.clientWidth<680?20:18);
    const height=overview?34:(scene.clientWidth<680?10:8.5);
    const eye=[player.x+Math.sin(yaw)*distance*.74,height,player.z+Math.cos(yaw)*distance*.74];
    viewProjection=multiply(perspective(Math.PI/3,scene.clientWidth/scene.clientHeight,.1,180),lookAt(eye,[player.x,overview?0:1.2,player.z]));
    gl.clearColor(.12,.24,.32,1);gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT);gl.useProgram(program);gl.uniformMatrix4fv(uniform,false,viewProjection);
    gl.uniform3f(translationUniform,0,0,0);draw(staticMesh);
    // The president is represented in the world by the gold figure at the camera focus.
    gl.uniform3f(translationUniform,player.x,0,player.z);draw(avatarMesh);
    for(const item of labels) {
      if(item.node.dataset.id==='current-human'){item.x=player.x;item.z=player.z;}
      const p=project(item.x,item.y,item.z);
      const mobile=scene.clientWidth<680;
      const nearby=Math.hypot(item.x-player.x,item.z-player.z)<10;
      const isPerson=item.node.classList.contains('person');
      const visible=p?.visible&&(!mobile||(overview?!isPerson:(p.y>130&&(!isPerson||nearby))));
      item.node.style.display=visible?'block':'none';
      if(visible) {item.node.style.left=`${p.x}px`;item.node.style.top=`${p.y}px`;}
    }
  }
  let nearest={title:'중앙 광장',dist:Infinity};
  for(const site of sites) {const dist=Math.hypot(site.x-player.x,site.z-player.z);if(dist<nearest.dist)nearest={title:site.title,dist};}
  $('#location').textContent=nearest.dist<10?nearest.title:'중앙 광장';
  requestAnimationFrame(render);
}

function getTarget(id) {
  const site=sites.find(s=>s.id===id);
  if(site)return {x:site.x,z:site.z+5};
  const pos=peoplePositions[id];return pos?{x:pos[0],z:pos[1]+2.5}:null;
}
function showDetail(id) {
  selected=id;destination=null;
  const office=officeById[id],person=personById[id];
  if(!office&&!person)return;
  const independent=person?.independent;
  $('#detail-overline').textContent=office?'MINISTRY / OFFICE':independent?'INDEPENDENT OVERSIGHT':'GOVERNMENT / PERSON';
  $('#detail-title').textContent=office?.title??person.name;
  $('#detail-subtitle').textContent=office?'정부 부처':person.title;
  $('#detail-description').textContent=office?.cardDescription??person.cardDescription;
  const count=office?DATA.people.filter(p=>p.officeId===office.id).length:0;
  $('#detail-meta').textContent=office?`현직 ${count}명 · ${office.responsibility}`:`현직 · 소속 ${person.officeId?officeById[person.officeId].title:independent?'독립 감사실':'국무총리실'}`;
  detail.classList.remove('hidden');
  document.querySelectorAll('[data-id]').forEach(el=>el.classList.toggle('active',el.dataset.id===id));
  document.querySelectorAll('.world-label').forEach(el=>el.classList.toggle('selected',el.dataset.id===id));
}
function label(id,title,eyebrow,x,y,z,kind='site') {
  const node=document.createElement('button');node.type='button';node.className=`world-label ${kind}`;node.dataset.id=id;
  node.innerHTML=`<small>${eyebrow}</small><strong>${title}</strong>`;
  node.addEventListener('click',()=>showDetail(id));labelsHost.append(node);labels.push({node,x,y,z});
  if(id==='current-human'){node.tabIndex=-1;node.setAttribute('aria-hidden','true');}
}
for(const site of sites.slice(0,4)) label(site.id,site.title,site.eyebrow,site.x,6.9,site.z);
for(const person of DATA.people) {const [x,z]=peoplePositions[person.id];label(person.id,person.name,person.title,x,2.85,z,'person');}
label('current-human','나 · 대통령','YOU ARE HERE',player.x,2.9,player.z,'player');
const navSites=[sites[0],sites[1],sites[2],sites[3]];
for(const site of navSites) {
  const count=DATA.people.filter(p=>p.officeId===site.id).length || 1;
  const button=document.createElement('button');button.className='district';button.dataset.id=site.id;
  button.innerHTML=`<span class="district-swatch" style="background:${site.color}">${site.id==='oversight.inspector'?'◇':site.id==='memory.information'?'▤':site.id==='execution.operations'?'⌘':'✦'}</span><span class="district-copy"><strong>${site.title}</strong><small>${count}명 · ${site.eyebrow}</small></span><span class="district-arrow">↗</span>`;
  button.addEventListener('click',()=>showDetail(site.id));districtsHost.append(button);
}
for(const person of DATA.people) {
  const button=document.createElement('button');button.className='person-chip';button.dataset.id=person.id;
  button.innerHTML=`<b>${person.name}</b><small>${person.title}</small>`;
  button.addEventListener('click',()=>showDetail(person.id));peopleHost.append(button);
}
$('#detail-close').addEventListener('click',()=>{detail.classList.add('hidden');selected=null;document.querySelectorAll('.active,.selected').forEach(el=>el.classList.remove('active','selected'));});
$('#view-toggle').addEventListener('click',()=>{overview=!overview;$('#view-toggle').innerHTML=overview?'탐험 시점으로 <span>↙</span>':'전체 지도 보기 <span>↗</span>';});
$('#travel').addEventListener('click',()=>{destination=getTarget(selected);if(scene.clientWidth<680)detail.classList.add('hidden');});
window.addEventListener('keydown',e=>{if(['w','a','s','d','arrowup','arrowdown','arrowleft','arrowright'].includes(e.key.toLowerCase())){keys.add(e.key.toLowerCase());e.preventDefault();}if(e.key==='Escape')$('#detail-close').click();});
window.addEventListener('keyup',e=>keys.delete(e.key.toLowerCase()));
window.addEventListener('blur',()=>keys.clear());
let drag=null;
canvas.addEventListener('pointerdown',e=>{drag={x:e.clientX,y:e.clientY,yaw};canvas.setPointerCapture(e.pointerId);});
canvas.addEventListener('pointermove',e=>{if(drag)yaw=drag.yaw+(e.clientX-drag.x)*.007;});
canvas.addEventListener('pointerup',()=>drag=null);
canvas.addEventListener('pointercancel',()=>drag=null);
document.querySelectorAll('[data-move]').forEach(button=>{
  const mapping={forward:'w',backward:'s',left:'a',right:'d'};
  button.addEventListener('pointerdown',e=>{e.preventDefault();keys.add(mapping[button.dataset.move]);button.setPointerCapture(e.pointerId);});
  for(const event of ['pointerup','pointercancel','lostpointercapture'])button.addEventListener(event,()=>keys.delete(mapping[button.dataset.move]));
});
requestAnimationFrame(render);
