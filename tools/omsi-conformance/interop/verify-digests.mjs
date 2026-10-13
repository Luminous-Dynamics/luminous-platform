#!/usr/bin/env node
// Independent JavaScript probe for the OMSI 0.1 restricted canonical JSON profile.
// This is a digest interoperability check, not a signature or identity verifier.
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const HERE = dirname(fileURLToPath(import.meta.url));
const read = (name) => JSON.parse(readFileSync(resolve(HERE, '..', 'fixtures', name), 'utf8'));
const ACTION_DOMAIN = 'org.omsi.action-arguments/0.1';
const MANIFEST_DOMAIN = 'org.omsi.operation-manifest/0.1';

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function assertUnicodeScalars(text, where) {
  for (let i = 0; i < text.length; i += 1) {
    const unit = text.charCodeAt(i);
    if (unit >= 0xD800 && unit <= 0xDBFF) {
      const next = text.charCodeAt(i + 1);
      if (!(next >= 0xDC00 && next <= 0xDFFF)) {
        throw new Error('invalid Unicode surrogate at ' + where);
      }
      i += 1;
    } else if (unit >= 0xDC00 && unit <= 0xDFFF) {
      throw new Error('invalid Unicode surrogate at ' + where);
    }
  }
}

function canonical(value, where = '/') {
  if (value === null) return 'null';
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  if (typeof value === 'string') {
    assertUnicodeScalars(value, where);
    return JSON.stringify(value);
  }
  if (typeof value === 'number') {
    if (!Number.isSafeInteger(value) || Object.is(value, -0)) {
      throw new Error('unsupported number in OMSI canonical JSON at ' + where);
    }
    return String(value);
  }
  if (Array.isArray(value)) {
    return '[' + value.map((v, i) => canonical(v, where + '[' + i + ']')).join(',') + ']';
  }
  if (typeof value === 'object') {
    const keys = Object.keys(value);
    for (const key of keys) assertUnicodeScalars(key, where + '<key>');
    // ECMAScript's default string ordering compares UTF-16 code units, matching
    // the constrained profile's object-key ordering.
    keys.sort();
    return '{' + keys.map((key) => JSON.stringify(key) + ':' + canonical(value[key], where + key + '/')).join(',') + '}';
  }
  throw new Error('unsupported JSON value at ' + where);
}

function digest(value, domain) {
  assert(typeof domain === 'string' && domain.length > 0 && /^[\x01-\x7F]+$/.test(domain), 'digest domain must be non-empty ASCII without NUL');
  const bytes = 'OMSI-DIGEST/0.1\0' + domain + '\0' + canonical(value);
  return 'sha256:' + createHash('sha256').update(Buffer.from(bytes, 'utf8')).digest('hex');
}

const op = read('operation.json');
const grant = read('grant.json');
const receipt = read('receipt.json');
const computedArgs = digest(op.action.arguments, ACTION_DOMAIN);
const computedManifest = digest(op, MANIFEST_DOMAIN);
assert(computedArgs === op.action.arguments_digest, 'action-argument digest mismatch: ' + computedArgs);
assert(computedManifest === grant.subject_manifest_digest, 'grant manifest digest mismatch: ' + computedManifest);
assert(computedManifest === receipt.subject_manifest_digest, 'receipt manifest digest mismatch: ' + computedManifest);

// Frozen golden vectors guard against accidental coordinated changes to both
// fixtures and the Python implementation. This vector exercises UTF-16 key
// ordering: U+10000 sorts before U+E000 by code units.
assert(
  digest({ a: 1, b: 2 }, 'org.omsi.test/0.1') === 'sha256:ff23c8a29d8a7bfc4f2a70040ff3c208b5a28318980c3f45072858e015515533',
  'basic digest golden vector mismatch',
);
assert(
  digest({ '\uE000': 1, '\u{10000}': 2, escaped: 'line\nquote"slash/é\u2028' }, 'org.omsi.unicode/0.1') === 'sha256:4bfca28a0e227704c6be8799f6a087df5cc02fffd92a376de59de2c4fcd84f13',
  'Unicode/escaping golden vector mismatch',
);
assert(
  digest({ a: 1 }, 'org.omsi.action-arguments/0.1') !== digest({ a: 1 }, MANIFEST_DOMAIN),
  'semantic digest domains are not separated',
);
console.log('PASS: independent JavaScript digest vectors and operation/grant/receipt bindings');
