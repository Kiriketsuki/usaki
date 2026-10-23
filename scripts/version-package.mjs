#!/usr/bin/env node
// Convert the repository's CalVer VERSION file (YY.Major.Minor.Patch[Suffix])
// into an npm-legal semver and write it into package.json "version".
//
// Mapping:
//   Major.Minor.Patch  -> semver core
//   20YY (+ any suffix) -> prerelease metadata
//
//   26.4.0.0   -> 4.0.0-2026
//   26.4.0.1a  -> 4.0.1-2026.a
//
// This script is OPT-IN and inert by default: if package.json is absent it is a
// clean no-op (exit 0), so it is safe to run in a repo that ships no Node package.
// It is also idempotent: writing the same version twice produces no change.
//
// Modes:
//   semver        write the npm semver into package.json
//   internal      write the raw CalVer string into package.json
//   print-semver  print the npm semver to stdout (no file writes)

import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const rootDir = resolve(fileURLToPath(new URL('.', import.meta.url)), '..')
const versionFilePath = resolve(rootDir, 'VERSION')
const packageJsonPath = resolve(rootDir, 'package.json')

const CALVER_RE = /^([0-9]{2})\.([0-9]+)\.([0-9]+)\.([0-9]+)([a-zA-Z]*)$/

function readInternalVersion() {
  const version = readFileSync(versionFilePath, 'utf8').trim()

  if (!CALVER_RE.test(version)) {
    throw new Error(
      `Invalid VERSION format: ${version}. Expected YY.Major.Minor.Patch[Suffix].`,
    )
  }

  return version
}

function toSemver(version) {
  const match = version.match(CALVER_RE)

  if (!match) {
    throw new Error(
      `Cannot transform invalid internal version: ${version}. Expected YY.Major.Minor.Patch[Suffix].`,
    )
  }

  const [, year, major, minor, patch, suffix] = match
  let semver = `${Number(major)}.${Number(minor)}.${Number(patch)}-20${year}`

  if (suffix) {
    semver += `.${suffix}`
  }

  return semver
}

function readPackageVersion() {
  const packageJson = readFileSync(packageJsonPath, 'utf8')
  const match = packageJson.match(/"version"\s*:\s*"([^"]+)"/)
  return match ? match[1] : null
}

function writePackageVersion(version) {
  // Inert by default: with no package.json there is nothing to update.
  if (!existsSync(packageJsonPath)) {
    return
  }

  const current = readPackageVersion()

  // Idempotent: skip the write when the version is already correct.
  if (current === version) {
    return
  }

  const packageJson = readFileSync(packageJsonPath, 'utf8')

  if (current === null) {
    throw new Error(
      'package.json exists but has no "version" field to update.',
    )
  }

  const updatedPackageJson = packageJson.replace(
    /"version"\s*:\s*"[^"]+"/,
    `"version": "${version}"`,
  )

  writeFileSync(packageJsonPath, updatedPackageJson)
}

function main() {
  const mode = process.argv[2]
  const internalVersion = readInternalVersion()

  if (mode === 'semver') {
    writePackageVersion(toSemver(internalVersion))
    return
  }

  if (mode === 'internal') {
    writePackageVersion(internalVersion)
    return
  }

  if (mode === 'print-semver') {
    process.stdout.write(toSemver(internalVersion))
    return
  }

  throw new Error(`Unsupported mode: ${mode}. Use "semver", "internal", or "print-semver".`)
}

main()
