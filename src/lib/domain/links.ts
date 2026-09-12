export interface ParsedLink {
  external_url: string | null;
  external_provider: string | null;
  external_id: string | null;
  title?: string;
}
export function parseTaskLink(input: string): ParsedLink {
  const raw = input.trim();
  if (!raw)
    return { external_url: null, external_provider: null, external_id: null };
  const markdown = raw.match(/^\[([^\]]+)\]\((https?:\/\/[^\s]+)\)$/i);
  const url = new URL(markdown?.[2] ?? raw);
  if (
    !['https:', 'http:'].includes(url.protocol) ||
    url.username ||
    url.password
  )
    throw new Error('Use an HTTP or HTTPS link without credentials.');
  const parts = url.pathname.split('/').filter(Boolean);
  let provider: string | null = null;
  let id: string | null = null;
  if (
    parts.length === 2 &&
    parts[0] === 'browse' &&
    /^[A-Z][A-Z0-9_]*-\d+$/i.test(parts[1])
  ) {
    provider = 'jira';
    id = parts[1];
  } else if (url.hostname === 'app.asana.com') {
    if (
      parts.length === 3 &&
      parts[0] === '0' &&
      /^\d+$/.test(parts[1]) &&
      /^\d+$/.test(parts[2])
    ) {
      provider = 'asana';
      id = parts[2];
    } else if (parts[0] === '1') {
      const at = parts.indexOf('task');
      if (at > 0 && at === parts.length - 2 && /^\d+$/.test(parts[at + 1])) {
        provider = 'asana';
        id = parts[at + 1];
      }
    }
  } else if (url.hostname === 'app.clickup.com') {
    const at = parts.indexOf('t');
    if (
      (at === 0 || at === 1) &&
      parts.length === at + 2 &&
      /^[a-z0-9_-]+$/i.test(parts[at + 1])
    ) {
      provider = 'clickup';
      id = parts[at + 1];
    }
  }
  return {
    external_url: url.href,
    external_provider: provider,
    external_id: id,
    ...(markdown ? { title: markdown[1] } : {}),
  };
}
export function applyPastedLink(input: string, title: string) {
  const parsed = parseTaskLink(input);
  return { ...parsed, title: title.trim() ? title : (parsed.title ?? title) };
}
