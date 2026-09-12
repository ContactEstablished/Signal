import pdf from './fixtures/gateway-arch.pdf?url';
import png from './fixtures/latency-p95.png?url';
import { installFixtureAttachments } from './native/commands';
export async function installSeedAttachments() {
  if (!import.meta.env.DEV)
    throw new Error('Fixture files require development mode.');
  const files = [
    {
      id: 'f870c681-27a4-4d65-87be-000000000d01',
      filename: 'gateway-arch.pdf',
      mime: 'application/pdf',
      url: pdf,
    },
    {
      id: 'f870c681-27a4-4d65-87be-000000000d02',
      filename: 'latency-p95.png',
      mime: 'image/png',
      url: png,
    },
  ];
  const fixtures = await Promise.all(
    files.map(async ({ url, ...file }) => {
      const response = await fetch(url);
      if (!response.ok) throw new Error(`Could not load ${file.filename}`);
      return {
        ...file,
        bytes: Array.from(new Uint8Array(await response.arrayBuffer())),
      };
    }),
  );
  await installFixtureAttachments(fixtures);
}
