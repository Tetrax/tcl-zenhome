// Shaders GLSL de la clairière. Tout est procédural : aucune texture.
// Les couleurs sont manipulées en sRGB brut (ColorManagement désactivé dans scene.ts).

// Bloc commun inclus dans les shaders de sommets ET de fragments.
export const COMMON = /* glsl */ `
uniform float uTime;
uniform float uWind;
uniform float uClouds;
uniform float uMist;
uniform float uFogDensity;
uniform vec3 uFogColor;
uniform vec3 uGlow;
uniform vec3 uKeyDir;
uniform vec3 uKey;
uniform vec3 uAmbSky;
uniform vec3 uAmbGround;

float hash21(vec2 p){ p = fract(p * vec2(123.34, 456.21)); p += dot(p, p + 45.32); return fract(p.x * p.y); }
float vnoise(vec2 p){
  vec2 i = floor(p), f = fract(p); f = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash21(i), hash21(i + vec2(1, 0)), f.x), mix(hash21(i + vec2(0, 1)), hash21(i + vec2(1, 1)), f.x), f.y);
}
// Ombres de nuages qui glissent sur le sol (partagées sol / herbe).
float cloudShadow(vec2 xz){
  float n = vnoise(xz * 0.045 + vec2(uTime * 0.03, uTime * 0.012));
  return 1.0 - 0.38 * uClouds * smoothstep(0.45, 0.8, n);
}
vec3 hemi(vec3 n){ return mix(uAmbGround, uAmbSky, n.y * 0.5 + 0.5); }
// Brouillard exponentiel + brume basse, teinté par la lueur du soleil.
vec3 applyFog(vec3 c, vec3 world){
  vec3 v = world - cameraPosition; float d = length(v); v /= d;
  float f = 1.0 - exp(-pow(d * uFogDensity, 2.0));
  float m = uMist * exp(-max(world.y, 0.0) * 0.3) * (1.0 - exp(-d * 0.03));
  float k = clamp(1.0 - (1.0 - f) * (1.0 - m), 0.0, 1.0);
  vec3 fc = uFogColor + uGlow * pow(max(dot(v, uKeyDir), 0.0), 5.0) * 0.38;
  return mix(c, fc, k);
}
`;

// ---------- Sol ----------
export const GROUND_VERT = /* glsl */ `
${COMMON}
varying vec3 vWorld; varying vec3 vN;
void main(){
  vec4 w = modelMatrix * vec4(position, 1.0);
  vWorld = w.xyz; vN = normalize(mat3(modelMatrix) * normal);
  gl_Position = projectionMatrix * viewMatrix * w;
}`;
export const GROUND_FRAG = /* glsl */ `
${COMMON}
uniform vec3 uGrassBase; uniform vec3 uGrassTip;
varying vec3 vWorld; varying vec3 vN;
void main(){
  vec3 n = normalize(vN);
  float nz = vnoise(vWorld.xz * 0.16) * 0.6 + vnoise(vWorld.xz * 0.8) * 0.4;
  vec3 col = mix(uGrassBase * 0.75, uGrassTip * 0.8, nz);
  float ndl = clamp(dot(n, uKeyDir) * 0.65 + 0.35, 0.0, 1.0);
  vec3 light = hemi(n) + uKey * ndl * cloudShadow(vWorld.xz);
  gl_FragColor = vec4(applyFog(col * light, vWorld), 1.0);
}`;

// ---------- Herbe (instanciée, vent dans le vertex shader) ----------
export const GRASS_VERT = /* glsl */ `
${COMMON}
attribute vec3 aOffset;   // base de la lame (x, y sol, z)
attribute vec4 aParams;   // hauteur, largeur, phase, rotation
uniform vec3 uMouse;      // point survolé sur le sol (w = force dans uMouseK)
uniform float uMouseK;
uniform vec4 uPulse;      // centre x, z ; âge (s) ; force
varying float vT; varying float vTint; varying vec3 vWorld; varying vec3 vN; varying float vGlow;
void main(){
  float h = aParams.x, w = aParams.y, ph = aParams.z, yaw = aParams.w;
  float t = position.y;
  vec3 p = vec3(position.x * w, t * h, t * t * h * 0.22);       // légère courbure naturelle
  float c = cos(yaw), s = sin(yaw);
  p.xz = vec2(c * p.x + s * p.z, -s * p.x + c * p.z);
  vec3 wb = aOffset;

  // Vent : ondulation fine + grandes rafales qui traversent la clairière.
  vec2 dir = normalize(vec2(1.0, 0.35)); vec2 perp = vec2(-dir.y, dir.x);
  float along = dot(wb.xz, dir);
  float g = (sin(along * 0.09 - uTime * 0.9) * 0.5 + 0.5) * (sin(wb.x * 0.05 + wb.z * 0.07 + uTime * 0.23) * 0.5 + 0.5);
  float gust = g * g * 1.6;
  float wave = sin(along * 0.7 - uTime * 2.1 + ph) * 0.5 + sin(wb.x * 1.3 + wb.z * 0.9 - uTime * 3.1 + ph * 2.0) * 0.25;
  float amp = (0.08 + 0.5 * gust) * uWind;
  float bend = t * t;
  vec2 disp = (dir * amp * (0.6 + wave * 0.6) + perp * wave * 0.05 * uWind) * h * bend;
  float lift = 0.0;

  // Survol : l'herbe s'écarte du pointeur.
  vec2 dm = wb.xz - uMouse.xz; float dl = length(dm);
  float push = exp(-dl * dl / 5.5) * uMouseK;
  disp += (dm / (dl + 0.001)) * push * 0.9 * h * bend;
  lift += push * 0.25;

  // Onde de lumière quand la box est trouvée.
  vec2 dp = wb.xz - uPulse.xy; float rp = length(dp);
  float ring = exp(-pow((rp - uPulse.z * 13.0) * 0.4, 2.0)) * exp(-uPulse.z * 0.45) * uPulse.w;
  disp += (dp / (rp + 0.001)) * ring * 0.8 * h * bend;
  vGlow = ring;

  p.y *= 1.0 - lift - bend * length(disp) / h * 0.35;
  vec3 world = vec3(wb.x + p.x + disp.x, wb.y + p.y, wb.z + p.z + disp.y);
  vWorld = world; vT = t; vTint = fract(ph * 7.31);
  vN = normalize(vec3(-s * 0.6, 1.1, c * 0.6));
  gl_Position = projectionMatrix * viewMatrix * vec4(world, 1.0);
}`;
export const GRASS_FRAG = /* glsl */ `
${COMMON}
uniform vec3 uGrassBase; uniform vec3 uGrassTip;
varying float vT; varying float vTint; varying vec3 vWorld; varying vec3 vN; varying float vGlow;
void main(){
  vec3 base = mix(uGrassBase, uGrassTip, vT);
  base *= 0.8 + 0.4 * vTint;
  base = mix(base, base * vec3(1.35, 1.15, 0.6), smoothstep(0.75, 1.0, vTint) * 0.55);  // quelques brins dorés
  base *= mix(0.4, 1.0, vT);                                                            // occlusion au pied
  vec3 n = normalize(vN);
  float ndl = clamp(dot(n, uKeyDir) * 0.5 + 0.5, 0.0, 1.0);
  vec3 light = hemi(n) + uKey * ndl * cloudShadow(vWorld.xz);
  // Contre-jour : l'herbe s'embrase quand on regarde vers le soleil.
  vec3 view = normalize(vWorld - cameraPosition);
  float back = pow(clamp(dot(view, uKeyDir), 0.0, 1.0), 3.0) * vT;
  vec3 col = base * light + uKey * base * back * 1.6;
  col += vec3(0.55, 0.9, 1.0) * vGlow * vT * 0.9;
  gl_FragColor = vec4(applyFog(col, vWorld), 1.0);
}`;

// ---------- Arbres (ombrage à facettes par dérivées) ----------
export const TREE_VERT = /* glsl */ `
${COMMON}
attribute vec3 aCol; attribute float iHue;
varying vec3 vCol; varying vec3 vWorld; varying float vY;
void main(){
  vec4 wp = modelMatrix * instanceMatrix * vec4(position, 1.0);
  float sway = position.y * position.y;
  float ph = wp.x * 0.07 + wp.z * 0.05;
  wp.x += sin(uTime * 0.7 + ph) * 0.35 * sway * uWind;
  wp.z += cos(uTime * 0.5 + ph * 1.3) * 0.2 * sway * uWind;
  vWorld = wp.xyz; vY = position.y;
  vCol = aCol * (0.8 + 0.45 * iHue) * mix(vec3(1.0), vec3(1.2, 1.05, 0.7), step(0.85, iHue) * step(0.2, aCol.g));
  gl_Position = projectionMatrix * viewMatrix * wp;
}`;
export const TREE_FRAG = /* glsl */ `
${COMMON}
varying vec3 vCol; varying vec3 vWorld; varying float vY;
void main(){
  vec3 fn = normalize(cross(dFdx(vWorld), dFdy(vWorld)));
  vec3 view = normalize(vWorld - cameraPosition);
  if (dot(fn, view) > 0.0) fn = -fn;
  float ndl = clamp(dot(fn, uKeyDir) * 0.7 + 0.3, 0.0, 1.0);
  vec3 col = vCol * (0.65 + 0.7 * vY);
  vec3 light = hemi(fn) + uKey * ndl * 0.9;
  // Liseré lumineux côté soleil.
  float rim = pow(clamp(dot(view, uKeyDir), 0.0, 1.0), 4.0) * pow(1.0 - abs(dot(fn, view)), 2.0);
  col = col * light + uKey * rim * 0.7;
  gl_FragColor = vec4(applyFog(col, vWorld), 1.0);
}`;

// ---------- Ciel : dégradé 3 teintes, soleil, lune, étoiles, nuages ----------
export const SKY_VERT = /* glsl */ `
varying vec3 vDir;
void main(){
  vDir = position;
  vec4 p = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  gl_Position = p.xyww;   // toujours au fond
}`;
export const SKY_FRAG = /* glsl */ `
${COMMON}
uniform vec3 uTop; uniform vec3 uMid; uniform vec3 uHorizon;
uniform vec3 uSunDir; uniform float uSunDisc; uniform float uMoon; uniform float uStars;
varying vec3 vDir;
const vec3 MOON = vec3(-0.3, 0.43, -0.85);
float fbm(vec2 p){ float a = 0.5, s = 0.0; for (int i = 0; i < 4; i++){ s += a * vnoise(p); p = p * 2.03 + 11.7; a *= 0.5; } return s; }
float starLayer(vec2 uv, float sc, float r, float seed){
  vec2 g = uv * sc; vec2 id = floor(g); vec2 f = fract(g) - 0.5;
  float h = hash21(id + seed);
  vec2 off = (vec2(hash21(id + 3.1 + seed), hash21(id + 7.7 + seed)) - 0.5) * 0.6;
  float d = length(f - off);
  float tw = 0.65 + 0.35 * sin(uTime * (1.0 + h * 4.0) + h * 60.0);
  return step(0.9, h) * smoothstep(r, 0.0, d) * (0.4 + (h - 0.9) * 6.0) * tw;
}
void main(){
  vec3 d = normalize(vDir);
  float h = clamp(d.y, 0.0, 1.0);
  vec3 col = mix(uHorizon, uMid, smoothstep(0.0, 0.22, h));
  col = mix(col, uTop, smoothstep(0.12, 0.75, h));
  float sd = max(dot(d, uSunDir), 0.0);
  col += uGlow * pow(sd, 5.0) * 0.38 * exp(-h * 3.0);                       // lueur d'horizon (raccord avec le brouillard)
  col += uGlow * (pow(sd, 48.0) * 0.3 + pow(sd, 400.0) * 0.9) * uSunDisc;  // halo solaire
  col += vec3(1.0, 0.97, 0.9) * smoothstep(0.99935, 0.99965, sd) * uSunDisc * 1.5;

  // Nuages doux (plan projeté).
  if (d.y > 0.01){
    vec2 cuv = d.xz / (d.y + 0.2);
    float c = smoothstep(0.52, 0.86, fbm(cuv * 0.8 + vec2(uTime * 0.008, 3.0))) * uClouds * smoothstep(0.01, 0.2, d.y);
    vec3 cc = uHorizon * 1.12 + uKey * 0.12 + uGlow * pow(sd, 4.0) * 0.6;
    col = mix(col, cc, c * 0.65);
  }
  // Étoiles, Voie lactée et lune.
  float up = smoothstep(0.0, 0.18, d.y);
  if (uStars > 0.01 && d.y > 0.0){
    vec2 uv = vec2(atan(d.z, d.x), asin(clamp(d.y, -1.0, 1.0)));
    float s = starLayer(uv, 75.0, 0.2, 0.0) + starLayer(uv, 140.0, 0.28, 9.0) * 0.8;
    float band = exp(-pow(dot(d, normalize(vec3(0.35, 0.75, 0.55))) * 3.5, 2.0));
    float mw = band * (0.35 + 0.65 * fbm(uv * 6.0)) * 0.22;
    col += (vec3(0.85, 0.92, 1.0) * s + vec3(0.45, 0.55, 0.9) * mw) * uStars * up;
  }
  float md = dot(d, normalize(MOON));
  col += uMoon * (vec3(0.95, 0.97, 1.0) * smoothstep(0.99972, 0.99982, md) * 1.3 + vec3(0.5, 0.65, 1.0) * (pow(max(md, 0.0), 60.0) * 0.28 + pow(max(md, 0.0), 8.0) * 0.07));
  gl_FragColor = vec4(col, 1.0);
}`;

// ---------- Rayons de lumière : plans additifs qui pivotent vers la caméra ----------
export const RAY_VERT = /* glsl */ `
${COMMON}
varying vec2 vUv; varying float vSeed; varying vec3 vWorld;
void main(){
  vec3 axis = normalize(modelMatrix[1].xyz);
  float len = length(modelMatrix[1].xyz), wid = length(modelMatrix[0].xyz);
  vec3 base = modelMatrix[3].xyz;
  vec3 right = normalize(cross(axis, cameraPosition - (base + axis * len * 0.5)));
  vec3 wp = base + axis * position.y * len + right * position.x * wid;
  vUv = vec2(position.x + 0.5, position.y);
  vSeed = fract(sin(dot(base.xz, vec2(12.9898, 78.233))) * 43758.5453);
  vWorld = wp;
  gl_Position = projectionMatrix * viewMatrix * vec4(wp, 1.0);
}`;
export const RAY_FRAG = /* glsl */ `
${COMMON}
uniform float uRays;
varying vec2 vUv; varying float vSeed; varying vec3 vWorld;
void main(){
  float x = vUv.x;
  float st = 0.5 + 0.5 * sin(x * 37.0 + vSeed * 40.0 + uTime * 0.15);
  st *= 0.5 + 0.5 * sin(x * 13.0 - vSeed * 9.0 + uTime * 0.1 + 1.0);
  st = pow(st, 1.4);
  float edge = smoothstep(0.0, 0.4, x) * smoothstep(1.0, 0.6, x);
  float along = smoothstep(0.0, 0.12, vUv.y) * smoothstep(1.0, 0.45, vUv.y);
  float d = length(cameraPosition - vWorld);
  float near = smoothstep(4.0, 18.0, d);
  float far = exp(-d * 0.012);
  float flick = 0.75 + 0.25 * sin(uTime * 0.4 + vSeed * 20.0);
  float a = st * edge * along * near * far * flick * uRays * 0.24;
  gl_FragColor = vec4(mix(uKey, uGlow, 0.35) * 1.4 + 0.04, a);
}`;

// ---------- Particules (pollen / lucioles) : boîte qui se répète en boucle ----------
export const SPRITE_VERT = /* glsl */ `
${COMMON}
attribute vec4 aSeed;
uniform vec3 uBoxMin; uniform vec3 uBoxSize; uniform vec3 uDrift;
uniform float uSize; uniform float uWander; uniform float uBlink; uniform float uScale;
varying float vA; varying float vDist;
void main(){
  vec3 s = aSeed.xyz; float t = uTime;
  vec3 b = s * uBoxSize + uDrift * t * (0.6 + aSeed.w * 0.8);
  b += uWander * vec3(sin(t * 0.5 + s.y * 60.0), 0.5 * sin(t * 0.7 + s.z * 50.0), cos(t * 0.45 + s.x * 55.0));
  vec3 q = mod(b, uBoxSize); vec3 e = q / uBoxSize;
  vec3 p = uBoxMin + q;
  float edge = smoothstep(0.0, 0.08, e.x) * smoothstep(1.0, 0.92, e.x) * smoothstep(0.0, 0.08, e.z) * smoothstep(1.0, 0.92, e.z) * smoothstep(0.0, 0.12, e.y) * smoothstep(1.0, 0.88, e.y);
  float bl = uBlink > 0.5 ? pow(0.5 + 0.5 * sin(t * (0.7 + aSeed.w * 1.3) + s.x * 80.0), 2.5) : (0.6 + 0.4 * sin(t * (1.0 + aSeed.w * 2.0) + s.z * 70.0));
  vA = edge * bl;
  vec4 mv = viewMatrix * vec4(p, 1.0);
  vDist = -mv.z;
  gl_PointSize = clamp(uSize * (0.6 + aSeed.w * 0.8) * uScale / max(-mv.z, 0.1), 1.0, 80.0);
  gl_Position = projectionMatrix * mv;
}`;
export const SPRITE_FRAG = /* glsl */ `
${COMMON}
uniform vec3 uColor; uniform float uAmount;
varying float vA; varying float vDist;
void main(){
  float d = length(gl_PointCoord - 0.5) * 2.0;
  if (d > 1.0) discard;
  float a = exp(-d * d * 5.0) * vA * uAmount * exp(-vDist * uFogDensity * 1.1);
  vec3 col = mix(uColor, vec3(1.0), pow(1.0 - d, 4.0) * 0.7);
  gl_FragColor = vec4(col, a);
}`;

// ---------- Ondes du signal (disque au sol) ----------
export const WAVE_VERT = /* glsl */ `
varying vec2 vP;
void main(){ vP = position.xy; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }`;
export const WAVE_FRAG = /* glsl */ `
${COMMON}
uniform float uSearch; uniform float uBurst; uniform float uSpot; uniform vec3 uColor;
varying vec2 vP;
void main(){
  float R = 17.0; float r = length(vP);
  float a = 0.0;
  for (int i = 0; i < 3; i++){
    float ph = fract(uTime * 0.2 + float(i) / 3.0);
    a += exp(-pow((r - ph * R) / (0.3 + ph * 0.6), 2.0)) * (1.0 - ph) * (1.0 - ph);
  }
  a *= uSearch * (0.75 + 0.25 * sin(atan(vP.y, vP.x) * 18.0 + uTime));
  a += exp(-pow((r - uBurst * R * 1.15) / 0.9, 2.0)) * (1.0 - uBurst) * step(0.001, uBurst) * 1.4;
  a += exp(-r * r * 0.18) * uSpot;
  a *= smoothstep(R, R * 0.7, r);
  gl_FragColor = vec4(uColor, clamp(a, 0.0, 1.0) * 0.8);
}`;

// ---------- Orbe lumineux (luciole-guide puis point de la box) ----------
export const ORB_VERT = /* glsl */ `
uniform float uSizeM; uniform float uScale;
void main(){
  vec4 mv = modelViewMatrix * vec4(0.0, 0.0, 0.0, 1.0);
  gl_PointSize = clamp(uSizeM * uScale / max(-mv.z, 0.1), 4.0, 400.0);
  gl_Position = projectionMatrix * mv;
}`;
export const ORB_FRAG = /* glsl */ `
uniform float uLevel; uniform vec3 uColor; uniform float uTime;
void main(){
  vec2 p = gl_PointCoord * 2.0 - 1.0; float d = length(p);
  float halo = exp(-d * d * 4.0) * 0.55;
  float core = exp(-d * d * 60.0) * 1.3;
  float flare = (exp(-abs(p.x) * 26.0) * exp(-abs(p.y) * 3.2) + exp(-abs(p.y) * 26.0) * exp(-abs(p.x) * 3.2)) * 0.55;
  float a = (halo + core + flare) * uLevel * (0.92 + 0.08 * sin(uTime * 3.0)) * smoothstep(1.0, 0.7, d);
  gl_FragColor = vec4(mix(uColor, vec3(1.0), clamp(core * 0.6, 0.0, 1.0)), clamp(a, 0.0, 1.0));
}`;
