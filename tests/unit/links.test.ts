import { it, expect } from 'vitest';
import { parseTaskLink, applyPastedLink } from '../../src/lib/domain/links';
it('detects supported providers locally and preserves user titles', () => {
  expect(
    parseTaskLink('https://jira.example.com/browse/ATL-512').external_id,
  ).toBe('ATL-512');
  expect(
    parseTaskLink('https://app.asana.com/0/123/1189').external_provider,
  ).toBe('asana');
  expect(parseTaskLink('https://app.clickup.com/team/t/86c').external_id).toBe(
    '86c',
  );
  expect(
    applyPastedLink(
      '[Pasted title](https://jira.example.com/browse/ATL-512)',
      'My title',
    ).title,
  ).toBe('My title');
  expect(
    applyPastedLink(
      '[Pasted title](https://jira.example.com/browse/ATL-512)',
      '',
    ).title,
  ).toBe('Pasted title');
  expect(
    applyPastedLink('https://jira.example.com/browse/ATL-512', '').title,
  ).toBe('');
  expect(
    parseTaskLink('https://app.clickup.com.evil.test/t/86c').external_provider,
  ).toBeNull();
  for (const url of [
    'javascript:alert(1)',
    'file:///test',
    'https://name:password@example.com',
  ])
    expect(() => parseTaskLink(url)).toThrow();
});
