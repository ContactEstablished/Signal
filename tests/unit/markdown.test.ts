// @vitest-environment jsdom
import { it, expect } from 'vitest';
import { renderMarkdown } from '../../src/lib/domain/markdown';
it('renders Markdown while stripping active HTML, remote images and unsafe links', () => {
  const html = renderMarkdown(
    '**Safe**\n\n[web](https://example.com)\n\n<img src="https://remote.test/x" onerror="alert(1)"><script>alert(1)</script><iframe src="https://evil.test"></iframe><a href="javascript:alert(1)">bad</a>',
  );
  expect(html).toContain('<strong>Safe</strong>');
  expect(html).toContain('href="https://example.com"');
  expect(html).not.toMatch(/<script|<img|<iframe|onerror|javascript:/i);
});
