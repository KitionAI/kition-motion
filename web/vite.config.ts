import { fileURLToPath } from 'node:url'
import react from '@vitejs/plugin-react'
import { defineConfig, type Plugin } from 'vite'

/**
 * The development host's workspace over HTTP, as Kition lends a project's folder: `dev.html`
 * puts each file of its in-memory workspace here (PUT), and the engine reads footage from here
 * with ranged GETs (`document.lend`). Only while developing (`vite` and the browser tests).
 */
function devWorkspace(): Plugin {
  const files = new Map<string, Buffer>()
  return {
    name: 'motion-dev-workspace',
    apply: 'serve',
    configureServer(server) {
      server.middlewares.use('/__workspace', (request, response) => {
        const path = decodeURIComponent((request.url ?? '/').split('?')[0]).replace(/^\/+/, '')
        if (request.method === 'PUT') {
          const chunks: Buffer[] = []
          request.on('data', (chunk: Buffer) => chunks.push(chunk))
          request.on('end', () => {
            files.set(path, Buffer.concat(chunks))
            response.statusCode = 204
            response.end()
          })
          return
        }
        const bytes = files.get(path)
        if (!bytes || (request.method !== 'GET' && request.method !== 'HEAD')) {
          response.statusCode = 404
          response.end()
          return
        }
        const range = /^bytes=(\d+)-(\d*)$/.exec(request.headers.range ?? '')
        const start = range ? Math.min(Number(range[1]), bytes.length) : 0
        const end = range && range[2] ? Math.min(Number(range[2]) + 1, bytes.length) : bytes.length
        response.statusCode = range ? 206 : 200
        response.setHeader('Accept-Ranges', 'bytes')
        response.setHeader('Content-Length', String(request.method === 'HEAD' ? bytes.length : end - start))
        if (range) response.setHeader('Content-Range', `bytes ${start}-${end - 1}/${bytes.length}`)
        response.end(request.method === 'HEAD' ? undefined : bytes.subarray(start, end))
      })
    },
  }
}

// The kit package is served from wherever a host puts it, so every asset path is relative.
export default defineConfig({
  base: './',
  plugins: [react(), devWorkspace()],
  resolve: {
    // @kition/sdk may be linked from kition-sdk (KITION_LINK=1); one React for the page.
    dedupe: ['react', 'react-dom'],
  },
  optimizeDeps: {
    exclude: ['@kition/sdk'],
    // The published SDK's editor reaches a CommonJS shim (through Tessera's UI library); excluded
    // above, it is not scanned, so the shim is named here to be converted for the browser.
    include: ['@kition/sdk > use-sync-external-store/shim'],
  },
  worker: { format: 'es' },
  server: {
    port: 5188,
    fs: { allow: [fileURLToPath(new URL('..', import.meta.url)), fileURLToPath(new URL('../../kition-sdk', import.meta.url)), fileURLToPath(new URL('../../kition-tessera', import.meta.url))] },
  },
  build: {
    outDir: 'dist',
    rollupOptions: { input: { index: fileURLToPath(new URL('index.html', import.meta.url)), dev: fileURLToPath(new URL('dev.html', import.meta.url)) } },
  },
})
