#!/usr/bin/env node
// Verificações de consistência do projeto (Node 20, sem dependências):
//  1. package.json, src-tauri/Cargo.toml e src-tauri/tauri.conf.json com a mesma versão;
//  2. tauri.conf.json é JSON válido e tem a pubkey do updater;
//  3. se GITHUB_REF_NAME for uma tag vX.Y.Z, a tag é igual à versão;
//  4. nenhum arquivo do projeto contém expressões proibidas.
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join, relative, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

const raiz = resolve(fileURLToPath(new URL('.', import.meta.url)), '..')
const erros = []
const falha = (msg) => erros.push(msg)

function lerTexto(caminho) {
  try {
    return readFileSync(join(raiz, caminho), 'utf8')
  } catch (e) {
    falha(`${caminho}: não foi possível ler (${e.message})`)
    return null
  }
}

// --- versões ---
const versoes = {}

const pkgTxt = lerTexto('package.json')
if (pkgTxt !== null) {
  try {
    versoes['package.json'] = JSON.parse(pkgTxt).version
  } catch (e) {
    falha(`package.json: JSON inválido (${e.message})`)
  }
}

const cargoTxt = lerTexto('src-tauri/Cargo.toml')
if (cargoTxt !== null) {
  const secao = cargoTxt.split(/^\[/m).find((s) => s.startsWith('package]'))
  const m = secao && secao.match(/^version\s*=\s*"([^"]+)"/m)
  if (m) versoes['src-tauri/Cargo.toml'] = m[1]
  else falha('src-tauri/Cargo.toml: version não encontrada na seção [package]')
}

const confTxt = lerTexto('src-tauri/tauri.conf.json')
if (confTxt !== null) {
  try {
    const conf = JSON.parse(confTxt)
    versoes['src-tauri/tauri.conf.json'] = conf.version
    const pubkey = conf?.plugins?.updater?.pubkey
    if (typeof pubkey !== 'string' || pubkey.trim() === '') {
      falha('src-tauri/tauri.conf.json: plugins.updater.pubkey ausente ou vazia')
    }
  } catch (e) {
    falha(`src-tauri/tauri.conf.json: JSON inválido (${e.message})`)
  }
}

const SEMVER = /^\d+\.\d+\.\d+$/
for (const [arq, v] of Object.entries(versoes)) {
  if (typeof v !== 'string' || !SEMVER.test(v)) falha(`${arq}: versão inválida (${JSON.stringify(v)}), esperado X.Y.Z`)
}
const unicas = new Set(Object.values(versoes))
if (unicas.size > 1) {
  falha(
    'versões divergentes: ' +
      Object.entries(versoes)
        .map(([a, v]) => `${a}=${v}`)
        .join(', '),
  )
}

// --- tag ---
const ref = process.env.GITHUB_REF_NAME ?? ''
if (/^v\d+\.\d+\.\d+$/.test(ref)) {
  const versao = [...unicas][0]
  if (ref !== `v${versao}`) falha(`a tag ${ref} não coincide com a versão ${versao}`)
}

// --- expressões proibidas ---
const PROIBIDAS = ['Co-Authored-By', 'Generated with']
const IGNORAR_DIRS = new Set(['node_modules', 'target', 'dist', '.git', 'dist-update', 'releases'])
const IGNORAR_ARQS = new Set(['package-lock.json', 'check-versions.mjs'])

function* andar(dir) {
  for (const nome of readdirSync(dir)) {
    const caminho = join(dir, nome)
    const st = statSync(caminho)
    if (st.isDirectory()) {
      if (!IGNORAR_DIRS.has(nome)) yield* andar(caminho)
    } else if (!IGNORAR_ARQS.has(nome)) {
      yield caminho
    }
  }
}

for (const caminho of andar(raiz)) {
  const buf = readFileSync(caminho)
  if (buf.includes(0)) continue // binário
  const txt = buf.toString('utf8')
  for (const expr of PROIBIDAS) {
    if (txt.includes(expr)) falha(`${relative(raiz, caminho).split(sep).join('/')}: contém a expressão proibida "${expr}"`)
  }
}

if (erros.length) {
  console.error('Falha nas verificações de consistência:')
  for (const e of erros) console.error(`  - ${e}`)
  process.exit(1)
}
console.log(`Consistência OK (versão ${[...unicas][0]}${/^v\d+\.\d+\.\d+$/.test(ref) ? `, tag ${ref}` : ''}).`)
