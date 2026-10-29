// Shaders GLSL ES 3.00 (three préfixe version, précisions et alias texture2D/varying/gl_FragColor).
import { TEX_W } from "./layers";

const COMMON = /* glsl */ `
float hash(vec2 p){ p=fract(p*vec2(123.34,456.21)); p+=dot(p,p+45.32); return fract(p.x*p.y); }
float vnoise(vec2 p){
  vec2 i=floor(p), f=fract(p); f=f*f*(3.-2.*f);
  return mix(mix(hash(i),hash(i+vec2(1.,0.)),f.x), mix(hash(i+vec2(0.,1.)),hash(i+vec2(1.,1.)),f.x), f.y);
}
float fbm(vec2 p){ float a=.5,s=0.; for(int i=0;i<4;i++){ s+=a*vnoise(p); p=p*2.03+17.1; a*=.5; } return s/.9375; }
`;

// --- Ciel plein cadre : dégradé, bandes de nuages d'estampe, soleil/lune à halo en paliers, étoiles ---
export const skyVert = /* glsl */ `
varying vec2 vUv;
void main(){ vUv=position.xy*.5+.5; gl_Position=vec4(position.xy,1.,1.); }
`;
export const skyFrag = /* glsl */ `
uniform vec3 uSkyTop,uSkyMid,uSkyHor,uSunCol,uHaze;
uniform vec2 uSun,uMoonP,uShift;
uniform float uSunPow,uSunSize,uHalo,uMoonPow,uStars,uAspect,uTime;
varying vec2 vUv;
${COMMON}
void main(){
  vec2 p=vUv+uShift;
  float h=p.y;
  vec3 c=mix(uSkyHor,uSkyMid,smoothstep(.12,.55,h));
  c=mix(c,uSkyTop,smoothstep(.5,1.,h));

  // bandes de nuages étirées
  float band=smoothstep(.0,.15,h-.3)*(1.-smoothstep(.6,.85,h));
  float cm=smoothstep(.5,.66,fbm(vec2(p.x*2.2+uTime*.004,p.y*14.)))*band;
  c=mix(c,mix(uSkyHor,uSunCol,.35),cm*.4);

  // étoiles
  vec2 sp=p*vec2(uAspect,1.)*75.;
  vec2 id=floor(sp); vec2 fp=fract(sp)-.5;
  float rn=hash(id);
  vec2 off=(vec2(hash(id+3.1),hash(id+7.7))-.5)*.6;
  float st=step(.955,rn)*smoothstep(.1+.08*rn,.0,length(fp-off))*(.55+.45*sin(uTime*(1.+rn*3.)+rn*40.));
  c+=vec3(.9,.95,1.)*st*smoothstep(.3,.7,h)*uStars;

  // soleil : disque + halo + anneaux en paliers
  vec2 ds=(p-uSun)*vec2(uAspect,1.); float r=length(ds);
  float glow=exp(-r*r/(uHalo*uHalo))*.34;
  for(int i=1;i<=4;i++){ glow+=(1.-smoothstep(uSunSize*(1.+float(i)*.9)-.004,uSunSize*(1.+float(i)*.9)+.004,r))*.045; }
  c+=uSunCol*glow*uSunPow;
  c=mix(c,mix(uSunCol,vec3(1.),.6),(1.-smoothstep(uSunSize-.003,uSunSize+.003,r))*uSunPow);

  // lune : disque piqueté, halo bleuté
  vec2 dm=(p-uMoonP)*vec2(uAspect,1.); float rm=length(dm);
  float mg=exp(-rm*rm/.03)*.28;
  for(int i=1;i<=3;i++){ mg+=(1.-smoothstep(.05*(1.+float(i)*.7)-.004,.05*(1.+float(i)*.7)+.004,rm))*.05; }
  c+=vec3(.55,.65,1.)*mg*uMoonPow;
  vec3 mc=vec3(.94,.96,1.)*(1.-.13*smoothstep(.55,.8,fbm(dm*45.)));
  c=mix(c,mc,(1.-smoothstep(.05-.0025,.05+.0025,rm))*uMoonPow);

  c+=(hash(gl_FragCoord.xy)-.5)/255.; // dither anti-bandes
  gl_FragColor=vec4(c,1.);
}
`;

// --- Couche découpée : silhouette, perspective aérienne, grain, liseré, ombres longues, flou de profondeur ---
export const layerVert = /* glsl */ `
varying vec2 vUv; varying float vD;
void main(){
  vUv=uv;
  vec4 mv=viewMatrix*modelMatrix*vec4(position,1.);
  vD=-mv.z;
  gl_Position=projectionMatrix*mv;
}
`;
export const layerFrag = /* glsl */ `
uniform sampler2D uTex;
uniform vec2 uSize; uniform float uT,uAcc,uReceive,uGrain;
uniform vec3 uFar,uNear,uAccent,uHaze,uRim;
uniform float uRimAmt,uFog,uShadow,uFocus,uTanHalf,uResY;
uniform vec2 uSunDir; uniform float uSunSign;
varying vec2 vUv; varying float vD;
const float APERTURE=.0035, MAXBLUR=.013;
${COMMON}
void main(){
  float d=vD;
  float visH=2.*d*uTanHalf;
  float coc=min(MAXBLUR,APERTURE*abs(d-uFocus)/d);            // rayon de flou, en fraction de la hauteur d'écran
  vec2 ru=vec2(coc*visH)/uSize;
  float lod=max(0.,log2(max(ru.x*${TEX_W}.,1.))-1.);

  int n=coc<.0006?1:DOF_N;
  vec4 acc=vec4(0.);
  for(int i=0;i<DOF_N;i++){
    if(i>=n) break;
    float fi=float(i);
    float ang=fi*2.39996; float rad=sqrt((fi+.5)/float(n));
    vec4 s=textureLod(uTex,vUv+vec2(cos(ang),sin(ang))*rad*ru*step(1.5,float(n)),lod);
    acc+=vec4(s.rgb*s.a,s.a);
  }
  float a=acc.a/float(n);
  if(a<.003) discard;
  vec3 data=acc.rgb/max(acc.a,1e-4);

  float shade=.5+(data.r-.5)*(-uSunSign);
  vec3 col=mix(uFar,uNear,uT);
  col=mix(col,uAccent,uAcc);
  col*=.55+.9*shade;

  float fogF=1.-exp(-d*uFog);

  // grain papier (accroché à la couche), atténué quand c'est flou ou lointain
  vec2 gp=vUv*uSize*uGrain;
  float gr=(vnoise(gp)-.5)*.11+(hash(floor(gp*1.7))-.5)*.05+(vnoise(vec2(gp.x*.12,gp.y*2.2))-.5)*.05;
  col*=1.+gr*(1.-smoothstep(0.,.008,coc))*(1.-fogF*.6);

  // ombres longues : un pixel de sol est dans l'ombre si, vers le soleil, il croise de la végétation
  if(uReceive>.5 && uShadow>.01 && data.g<.5){
    float sh=0.; float sg=uSunSign>=0.?1.:-1.;
    for(int k=1;k<=12;k++){
      float dx=.25*pow(1.33,float(k));
      sh=max(sh,smoothstep(.15,.6,textureLod(uTex,vUv+vec2(sg*dx/uSize.x,.34*dx/uSize.y),2.5).g));
    }
    col=mix(col,col*vec3(.62,.55,.78),sh*uShadow*.65);
  }

  // liseré de lumière sur le bord tourné vers le soleil
  float wo=1.5*visH/uResY+coc*visH*.6;
  float a2=textureLod(uTex,vUv+uSunDir*wo/uSize,lod).a;
  float rim=a*(1.-smoothstep(0.,.85,a2));

  col=mix(col,uHaze,fogF);
  col+=uRim*rim*uRimAmt*(1.-fogF*.45)*(.55+.45*uT);
  gl_FragColor=vec4(col,a*smoothstep(.6,5.5,d));
}
`;

// --- Lac : dégradé, traits d'eau, reflet du soleil/de la lune, ondes concentriques du « signal » ---
export const waterFrag = /* glsl */ `
uniform vec3 uWaterA,uWaterB,uHaze,uSkyHor,uSunCol;
uniform vec2 uSun,uMoonP,uRipple,uRes;
uniform float uSunPow,uMoonPow,uGlitter,uTime,uSignal,uAspect,uFog,uWaterY,uSpeed;
varying vec3 vW; varying float vD;
${COMMON}
void main(){
  float dw=uWaterY-vW.y;                                    // profondeur sous la ligne d'eau
  if(dw < -.15*vnoise(vec2(vW.x*.18,3.))) discard;          // rive irrégulière
  vec2 sp=gl_FragCoord.xy/uRes;
  vec3 col=mix(uWaterA,uWaterB,1.-exp(-max(dw,0.)*.09));
  col=mix(col,uSkyHor,exp(-max(dw,0.)*.35)*.4);             // reflet de l'horizon

  // traits d'eau horizontaux, légèrement animés
  float t=uTime*uSpeed;
  float s1=vnoise(vec2(vW.x*.32+sin(t*.15+vW.y)*.5, vW.y*6.2+t*.08));
  float lines=smoothstep(.64,.74,s1);
  col=mix(col,uSkyHor,lines*.22);
  col=mix(col,uWaterB*.55,exp(-max(dw,0.)*.7)*.45*(1.-s1*.6)); // reflet sombre de la rive

  // reflet : colonne de paillettes sous le soleil et sous la lune
  float s2=vnoise(vec2(vW.x*1.1+t*.1, vW.y*9.));
  float dash=smoothstep(.55,.7,s2);
  float widen=.025+.06*clamp(dw/16.,0.,1.);
  float cs=exp(-pow((sp.x-uSun.x)*uAspect/widen,2.))*uSunPow;
  float cm=exp(-pow((sp.x-uMoonP.x)*uAspect/(widen*.7),2.))*uMoonPow;
  col+=uSunCol*dash*cs*uGlitter*.85;
  col+=vec3(.85,.9,1.)*dash*cm*uGlitter*1.3;

  // ondes du signal
  float ring=0.;
  for(int i=0;i<4;i++){
    float age=fract(uTime*.26+float(i)*.25);
    vec2 e=vec2((vW.x-uRipple.x)/1.,(vW.y-uRipple.y)/.2);
    float R=age*30.;
    float x=(length(e)-R)/.6;
    ring+=exp(-x*x)*(1.-age)*(1.-age);
  }
  col=mix(col,vec3(1.),clamp(ring,0.,1.)*uSignal*.9);

  col=mix(col,uHaze,1.-exp(-vD*uFog));
  gl_FragColor=vec4(col,1.);
}
`;

// --- Nappe de brume qui glisse ---
export const mistFrag = /* glsl */ `
uniform vec3 uHaze,uSunCol;
uniform float uMist,uTime,uSeed,uSpeed;
varying vec2 vUv; varying float vD; varying vec3 vW;
${COMMON}
void main(){
  float prof=1.-abs(vUv.y*2.-1.); prof=prof*prof*(3.-2.*prof);
  float n=fbm(vec2(vW.x*.045+uTime*uSpeed+uSeed*10.,vUv.y*2.4+uSeed));
  float a=smoothstep(.32,.78,n)*prof*uMist*.62;
  a*=smoothstep(2.,12.,vD);
  gl_FragColor=vec4(mix(uHaze,uSunCol,.18),a);
}
`;

// --- Poussières de lumière / lucioles (Points) ---
export const pointsVert = /* glsl */ `
attribute vec4 aSeed;
uniform float uTime,uSize,uResY,uTanHalf,uAmount,uBlink;
uniform vec3 uMin,uSpan; uniform vec2 uDrift;
varying float vA;
void main(){
  vec3 s=aSeed.xyz; float w=aSeed.w;
  vec3 p=uMin+s*uSpan;
  p.x+=sin(uTime*(.05+.07*w)+s.y*40.)*(1.+2.*w);
  p.y+=sin(uTime*(.07+.05*w)+s.x*30.)*.8;
  p.y=uMin.y+mod(p.y-uMin.y+uTime*uDrift.y*(.4+w),uSpan.y);
  p.x=uMin.x+mod(p.x-uMin.x+uTime*uDrift.x*(.4+w),uSpan.x);
  vec4 mv=viewMatrix*vec4(p,1.);
  float d=-mv.z;
  gl_Position=projectionMatrix*mv;
  gl_PointSize=clamp(uSize*(.5+w)*uResY/(2.*max(d,.5)*uTanHalf),1.5,60.);
  float tw=uBlink>.5 ? smoothstep(.15,.9,.5+.5*sin(uTime*(.6+w)+s.x*60.)) : .55+.45*sin(uTime*(.8+2.*w)+s.z*50.);
  vA=tw*smoothstep(.8,4.,d)*(1.-smoothstep(90.,140.,d))*uAmount;
}
`;
export const pointsFrag = /* glsl */ `
uniform vec3 uColor;
varying float vA;
void main(){
  vec2 q=gl_PointCoord*2.-1.;
  float a=exp(-dot(q,q)*3.5)*vA;
  if(a<.004) discard;
  gl_FragColor=vec4(uColor,a);
}
`;

// --- Oiseaux lointains : chevrons qui battent des ailes (Points) ---
export const birdVert = /* glsl */ `
attribute vec4 aSeed;
uniform float uTime,uSize,uResY,uTanHalf,uAmount;
uniform vec3 uMin,uSpan;
varying float vFlap; varying float vA;
void main(){
  vec3 s=aSeed.xyz; float w=aSeed.w;
  vec3 p=uMin+s*uSpan;
  p.x=uMin.x+mod(s.x*uSpan.x+uTime*(.9+w*.7),uSpan.x);
  p.y+=sin(uTime*.5+s.z*20.)*.8;
  vec4 mv=viewMatrix*vec4(p,1.);
  float d=-mv.z;
  gl_Position=projectionMatrix*mv;
  gl_PointSize=clamp(uSize*uResY/(2.*max(d,.5)*uTanHalf),2.,60.);
  vFlap=sin(uTime*(5.+w*3.)+s.z*30.);
  vA=uAmount;
}
`;
export const birdFrag = /* glsl */ `
uniform vec3 uColor;
varying float vFlap; varying float vA;
void main(){
  vec2 q=gl_PointCoord*2.-1.; q.y=-q.y;
  float f=-.1+abs(q.x)*(vFlap*.55+.15);
  float line=(1.-smoothstep(.07,.15,abs(q.y-f)))*step(abs(q.x),.95);
  float a=line*vA;
  if(a<.01) discard;
  gl_FragColor=vec4(uColor,a);
}
`;

// --- Lueur de la lampe de la cabane (additif) ---
export const glowFrag = /* glsl */ `
uniform float uBox,uPulse,uTime;
uniform vec3 uLamp;
varying vec2 vUv;
void main(){
  vec2 q=vUv*2.-1.; float r=length(q);
  float core=exp(-r*r*140.), halo=exp(-r*r*9.)*.4;
  float ring=uPulse>.001 ? exp(-pow((r-uPulse*.95)*10.,2.))*(1.-uPulse)*1.4 : 0.;
  float fl=.93+.07*sin(uTime*7.+sin(uTime*2.3)*3.);
  float a=clamp((core*1.3+halo)*uBox*fl+ring,0.,1.);
  if(a<.004) discard;
  gl_FragColor=vec4(uLamp,a);
}
`;
export const glowVert = /* glsl */ `
varying vec2 vUv;
void main(){ vUv=uv; gl_Position=projectionMatrix*viewMatrix*modelMatrix*vec4(position,1.); }
`;

// Lac et brume partagent ce vertex (position monde + profondeur)
export const worldVert = /* glsl */ `
varying vec2 vUv; varying float vD; varying vec3 vW;
void main(){
  vUv=uv;
  vec4 w=modelMatrix*vec4(position,1.);
  vW=w.xyz;
  vec4 mv=viewMatrix*w;
  vD=-mv.z;
  gl_Position=projectionMatrix*mv;
}
`;
