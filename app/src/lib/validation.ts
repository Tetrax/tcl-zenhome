// Validation côté interface (le cœur revalide). Même règle que Ipv4Addr : 4 nombres de 0 à 255, sans zéro superflu.
const OCTET = "(?:25[0-5]|2[0-4]\\d|1\\d\\d|[1-9]?\\d)";
const IPV4 = new RegExp(`^${OCTET}(?:\\.${OCTET}){3}$`);
/** Le cœur n'accepte que les adresses de réseau local : 10.x, 172.16 à 31.x, 192.168.x, ou link-local 169.254.x. */
export const ipLocale = (s: string): boolean => {
  const [a, b] = s.split(".").map(Number);
  return a === 10 || (a === 172 && b >= 16 && b <= 31) || (a === 192 && b === 168) || (a === 169 && b === 254);
};
export const ipValide = (s: string): boolean => IPV4.test(s);
