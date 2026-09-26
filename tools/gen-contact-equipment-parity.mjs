// SPDX-License-Identifier: GPL-3.0-or-later
// Record rendered web seat attachments and fitted V3 pole leaves. No port formulas.
import { register } from 'node:module';
import { execFileSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
register('./gen-rig-phase-resolve.mjs', import.meta.url);
const root = resolve(import.meta.dirname, '..');
const ref = join(root, 'reference/rowplay');
const pin = '173c6facbcedef419ad39168c5e3e642abb7e57e';
if (execFileSync('git', ['-C', ref, 'rev-parse', 'HEAD'], {encoding:'utf8'}).trim() !== pin) throw Error('reference pin mismatch');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const files = ['renderer3dRowAvatar.ts','renderer3dSkiAvatar.ts','renderer3dAvatarKit.ts','renderer3dAssets.ts','figurePose.ts','skiEquipment.ts','strokeModel.ts','motionGraph.ts','motion.ts','rowRig.ts','handGrip.ts'];
const sourceFileSha256s = {};
for (const file of files) sourceFileSha256s[file] = hash(await readFile(join(ref,'src/lib/replay',file)));
const url = file => pathToFileURL(join(ref,'src/lib/replay',file)).href;
const THREE = await import('three');
const {makeRowerAvatar} = await import(url('renderer3dRowAvatar.ts'));
const {makeSkierAvatar} = await import(url('renderer3dSkiAvatar.ts'));
const {fallbackStrokePose} = await import(url('strokeModel.ts'));
const {fetchReplayAssetTemplateLibrary,applyReplayAssetLibrary} = await import(url('renderer3dAssets.ts'));
const bytes = await readFile(join(ref,'static/replay-assets/rowplay-rigs-v3.glb'));
const library = await fetchReplayAssetTemplateLibrary(async()=>new Response(new Uint8Array(bytes)));
const samples=[];
for (const [sport,factory,steps] of [['rower',makeRowerAvatar,[0,380,760,1380]],['skierg',makeSkierAvatar,[1900,0,300,540,1100]]]) {
 const avatar=factory(0x3b82f6,false,1,16,'high');
 const scene=new THREE.Scene();scene.add(avatar.group);
 const named=name=>{const n=avatar.group.getObjectByName(name);if(!n)throw Error(name);return n;};
 if(sport==='skierg') applyReplayAssetLibrary(avatar.group,library);
 for(const step of steps){
  const pose=fallbackStrokePose(sport,step/2000*Math.PI*2,30);
  for(let warm=0;warm<10;warm++)avatar.animate(pose.phase,false,pose,step*3);
  avatar.resolveWorldContacts?.();scene.updateMatrixWorld(true);
  const position=n=>n.getWorldPosition(new THREE.Vector3()).toArray();
  if(sport==='rower')samples.push({sport,step,pose,carriageLocal:named('rower-seat-carriage').position.toArray(),carriage:position(named('rower-seat-carriage')),pelvis:position(avatar.v4Targets.pelvis)});
  else for(const side of ['left','right']){
   const hand=named('skierg-hand-'+side),tip=named('skierg-pole-tip-'+side),shaft=named('skierg-pole-shaft-'+side),grip=named('skierg-pole-grip-'+side);
   const leaf=n=>{if(!n.userData.authoredReplayAsset)throw Error('not fitted');n.geometry.computeBoundingBox();return {position:position(n),quaternion:n.getWorldQuaternion(new THREE.Quaternion()).toArray(),scale:n.getWorldScale(new THREE.Vector3()).toArray(),bounds:[n.geometry.boundingBox.min.toArray(),n.geometry.boundingBox.max.toArray()],vertices:Array.from(n.geometry.attributes.position.array)};};
   samples.push({sport,step,side,hand:position(hand),tip:position(tip),upperQuaternion:named('skierg-upper').getWorldQuaternion(new THREE.Quaternion()).toArray(),shaft:leaf(shaft),grip:leaf(grip)});
  }
 }
}
const out=process.argv[2]??join(root,'tests/fixtures/replay-contact-equipment-parity.json');
await writeFile(out,JSON.stringify({schema:'rowplay-qt.contact-equipment.v1',sourceCommit:pin,generatorVersion:'gen-contact-equipment-parity/1.0.0',sourceFileSha256s,assetSha256:hash(bytes),samples},null,1)+'\n');
console.log(out);
