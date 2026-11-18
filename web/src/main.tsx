import { createRoot } from 'react-dom/client'
import { connectHost, createKitStrings, loadPhrases, withPhrases } from '@kition/kit-editor'
import '@kition/kit-editor/styles.css'
import './app.css'
import { App } from './App'
import { CATALOGS, PHRASES } from './strings'
import manifest from '../../kit.json'
import './commandIcons'

const root = createRoot(document.getElementById('root')!)

// Opened outside a host, the page says where it belongs instead of waiting forever.
const alone = window.setTimeout(() => {
  const t = createKitStrings(CATALOGS, navigator.language)
  root.render(<div className="kui-alone">{t('noHost')}</div>)
}, 3000)

// `replica`: in a host that keeps replicas, a `.kmotion` keeps its KDM replica (identity,
// collaboration) across saves; elsewhere the editor opens and saves files as before.
void connectHost({ id: 'kit.motion', version: '0.1.0', capabilities: ['ai', 'replica'], brand: manifest.ui.brand, display: manifest.ui.display }).then(async (host) => {
  window.clearTimeout(alone)
  // Kition Motion ships no catalogs of its own; this editor's phrases translate its menus.
  const phrase = withPhrases(await loadPhrases({}, host.init.locale), PHRASES, host.init.locale)
  // No StrictMode: its doubled effects would open the document twice, in two engine workers.
  root.render(<App host={host} phrase={phrase} />)
})
