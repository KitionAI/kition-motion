// Takes the README's pictures of the Motion editor (assets/readme/*.png at the repository root):
// the editor in the development host, on the showcase sample "Lumen Studio Logo Reveal", at
// 1440 x 900, device scale 2, Classic display mode, light theme, English. Not a test; re-run it by
// hand when the editor changes:
//
//   cd web && pnpm install && pnpm run engine      # the engine and fonts in public/
//   node scripts/readme-shots.mjs [name ...]       # every shot, or only the named ones
//
// It needs kition-sdk (the shared runner, tools/readme-shots/) and kition-showcase (the sample)
// checked out beside this repository, or KITION_SDK and KITION_SHOWCASE naming them. Run it with
// no HTTP proxy set; CHROME may name a Chrome or Chromium executable. Every step is what a person
// does in the editor: clicks on the time ruler, the timeline's layers and the ribbon. When pngquant
// is installed, each picture is then shrunk with `pngquant --quality 70-90` (a README picture is at
// most 400 KB). Each picture also needs its `.attribution` sidecar (AGENTS.md §1), kept by hand.
import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const web = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const sdk = path.resolve(process.env.KITION_SDK ?? path.join(web, '..', '..', 'kition-sdk'))
const showcase = path.resolve(process.env.KITION_SHOWCASE ?? path.join(web, '..', '..', 'kition-showcase'))
const { takeShots } = await import(pathToFileURL(path.join(sdk, 'tools/readme-shots/shots.mjs')).href)

const SAMPLE = path.join(showcase, 'motion', 'lumen-studio-logo-reveal', 'lumen-studio-logo-reveal.kmotion')

/** Moves the current time with a click on the time ruler, `fraction` of the way along it. */
async function seek(page, kit, fraction) {
  const ruler = await kit.getByTestId('motion-ruler').boundingBox()
  await page.mouse.click(ruler.x + ruler.width * fraction, ruler.y + ruler.height / 2)
  await page.waitForTimeout(1500)
}

/** Selects a layer by a click on its row in the timeline. */
async function select(page, kit, name) {
  await kit.locator('[data-testid^="motion-layer-"]', { hasText: name }).first().click()
  await page.waitForTimeout(800)
}

const report = await takeShots({
  web,
  sample: SAMPLE,
  query: { modes: 'classic' },
  ready: '[data-testid="motion-status"]',
  settle: { open: 4000, shot: 1200 },
  shots: [
    {
      // The ident at 4 s, every beat in place; the wordmark layer selected, its Transform beside it.
      name: 'hero',
      steps: async (page, { kit }) => {
        await seek(page, kit, 0.8)
        await select(page, kit, 'LUMEN STUDIO')
      },
    },
    {
      // At 1.1 s the prism pops inside the ring: its Scale is keyframed (the filled diamond), and
      // the timeline shows every layer's keyframes.
      name: 'see-1',
      steps: async (page, { kit }) => {
        await seek(page, kit, 0.22)
        await select(page, kit, 'Prism')
      },
    },
    {
      // The ring selected and the ribbon's Effects menu open; the ring's gold glow is the Glow effect.
      name: 'see-2',
      steps: async (page, { kit }) => {
        await seek(page, kit, 0.8)
        await select(page, kit, 'Ring')
        await kit.getByTestId('motion-home-effects').click()
        await page.waitForTimeout(800)
      },
    },
    {
      // The ribbon's New layer menu: text, shape, solid, null and the other kinds of layer.
      name: 'see-3',
      steps: async (page, { kit }) => {
        await seek(page, kit, 0.8)
        await kit.getByTestId('motion-home-new-layer').click()
        await page.waitForTimeout(800)
      },
    },
    {
      // Render MP4: the composition rendered to an .mp4 beside the project, and the editor saying
      // so. The message fades, so the picture is taken soon after the host has the movie.
      name: 'see-4',
      steps: async (page, { kit }) => {
        await seek(page, kit, 0.8)
        await kit.getByTestId('motion-render').click()
        await page.waitForFunction(() => [...window.devHost.files.keys()].some((name) => name.endsWith('.mp4')), null, { timeout: 120_000 })
        await page.waitForTimeout(300)
      },
    },
  ],
})

// Shrinks the pictures when pngquant is there (it keeps a file it cannot make smaller).
const repo = path.resolve(web, '..')
for (const { file } of report) {
  try {
    execFileSync('pngquant', ['--quality', '70-90', '--force', '--skip-if-larger', '--output', file, file], { cwd: repo, stdio: 'ignore' })
  } catch (error) {
    if (error.code === 'ENOENT') break
  }
  console.log(`${file}  ${Math.ceil(fs.statSync(path.join(repo, file)).size / 1024)} KB`)
}
