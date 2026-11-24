import { createRoot } from 'react-dom/client'
import { connectHost, createKitStrings, loadPhrases, withPhrases, type Phrases } from '@kition/sdk/editor'
import '@kition/sdk/editor/styles.css'
import './app.css'
import { App } from './App'
import { CATALOGS, PHRASES } from './strings'
import manifest from '../../kit.json'
import './commandIcons'

/**
 * A numbered default name the phrases lack (`Animator 7`, `Light 2`, `Mask 3`) reads as its
 * translated base and the number (`动画制作工具 7`); any other text as the phrases have it.
 */
function numbered(phrase: Phrases): Phrases {
  return (english) => {
    const translated = phrase(english)
    if (translated !== english) return translated
    const match = /^(.*\S) (\d+)$/.exec(english)
    if (!match) return translated
    const base = phrase(match[1])
    return base === match[1] ? translated : `${base} ${match[2]}`
  }
}

const root = createRoot(document.getElementById('root')!)

// Opened outside a host, the page says where it belongs instead of waiting forever.
const alone = window.setTimeout(() => {
  const t = createKitStrings(CATALOGS, navigator.language)
  root.render(<div className="kui-alone">{t('noHost')}</div>)
}, 3000)

// `replica`: in a host that keeps replicas, a `.kmotion` keeps its KDM replica (identity,
// collaboration) across saves; elsewhere the editor opens and saves files as before.
void connectHost({ id: 'kit.motion', version: '0.1.3', capabilities: ['ai', 'replica'], brand: manifest.ui.brand, display: manifest.ui.display }).then(async (host) => {
  window.clearTimeout(alone)
  // Kition Motion ships no catalogs of its own; this editor's phrases translate its menus.
  const phrase = numbered(withPhrases(await loadPhrases({}, host.init.locale), PHRASES, host.init.locale))
  // No StrictMode: its doubled effects would open the document twice, in two engine workers.
  root.render(<App host={host} phrase={phrase} />)
})
