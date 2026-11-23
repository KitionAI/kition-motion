// The editor in each display mode, through the development host: node tests/modes.mjs [chromium|webkit]
// The sample opens with no layer selected, so Home's Pre-compose, Add Keyframe, Easy Ease, and
// Effects wait for a selection: the Single line row starts with five enabled buttons (`min: 5`).
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'
import { runModeSmoke } from '@kition/sdk/dev-host/mode-smoke'

await runModeSmoke({
  kit: 'kit.motion',
  config: new URL('../vite.config.ts', import.meta.url).pathname,
  browser: process.argv[2] === 'webkit' ? webkit : chromium,
  createServer,
  line: { tab: 'home', min: 5, primary: ['motion-home-new-layer', 'motion-cmd-layer.newText', 'motion-cmd-layer.newShape', 'motion-home-add-key', 'motion-home-effects', 'motion-cmd-comp.settings'] },
})
