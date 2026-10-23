// Checks every .glb file in a folder with the Khronos glTF Validator, the
// official checker for glTF files. Prints each file's counts and every
// error and warning, and fails if any file has an error. Warnings are allowed; notes and hints are only counted (every
// file has a few: its tangents are there for normal maps painted later).
//
// The validator comes from npm (package `gltf-validator`, Apache-2.0); it
// is a library, not a command, hence this small script. CI installs it
// into a folder of its own and points NODE_PATH there:
//
//   npm install --prefix /tmp/validator gltf-validator@2.0.0-dev.3.10
//   NODE_PATH=/tmp/validator/node_modules node tools/gltf-check/validate.cjs <folder>
'use strict';

const fs = require('fs');
const path = require('path');
const validator = require('gltf-validator');

// The validator's severities: 0 error, 1 warning, 2 note, 3 hint.
const SEVERITY = ['ERROR', 'WARNING', 'NOTE', 'HINT'];

async function main() {
    const folder = process.argv[2];
    if (!folder) {
        console.error('Usage: node validate.cjs <folder of .glb files>');
        return 2;
    }
    const files = fs.readdirSync(folder).filter((f) => f.endsWith('.glb')).sort();
    if (files.length === 0) {
        console.error(`No .glb files in ${folder}: the export step wrote nothing.`);
        return 1;
    }
    let failed = 0;
    for (const name of files) {
        const bytes = new Uint8Array(fs.readFileSync(path.join(folder, name)));
        let report;
        try {
            report = await validator.validateBytes(bytes, { uri: name, maxIssues: 0 });
        } catch (error) {
            console.log(`::error title=glTF Validator::${name} could not be read: ${error}`);
            failed += 1;
            continue;
        }
        const issues = report.issues;
        console.log(
            `${name}: ${issues.numErrors} errors, ${issues.numWarnings} warnings, ` +
                `${issues.numInfos} notes, ${issues.numHints} hints`,
        );
        for (const message of issues.messages) {
            const where = message.pointer ? ` at ${message.pointer}` : '';
            const line = `  ${SEVERITY[message.severity] || message.severity} ${message.code}${where}: ${message.message}`;
            if (message.severity === 0) {
                console.log(`::error title=glTF Validator (${name})::${line.trim()}`);
            } else if (message.severity === 1) {
                console.log(`::warning title=glTF Validator (${name})::${line.trim()}`);
            }
        }
        if (issues.numErrors > 0) {
            failed += 1;
        }
    }
    if (failed > 0) {
        console.log(`${failed} of ${files.length} files have errors.`);
        return 1;
    }
    console.log(`All ${files.length} files are valid glTF.`);
    return 0;
}

main().then(
    (code) => process.exit(code),
    (error) => {
        console.error(error);
        process.exit(1);
    },
);
