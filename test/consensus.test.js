'use strict';

const worker = require('../index').consensus;

const test = require('tape');
const path = require('path');
const fs = require('fs');

const db = require('./lib/db');
db.init(test);

test('consensus - sources argument error', (t) => {
    t.throws(() => worker(), /sources argument is required/);
    t.end();
});

test('consensus - test_set argument error', (t) => {
    t.throws(() => worker({
        sources: [path.resolve(__dirname, './fixtures/dc-persistent.geojson')]
    }), /test_set argument is required/);
    t.end();
});

test('consensus', (t) => {
    // Ensure files don't exist before test
    try {
        fs.unlinkSync('/tmp/error-sources');
        fs.unlinkSync('/tmp/error-test-set');
    } catch (err) {
        console.error('ok - cleaned tmp files');
    }

    const results = worker({
        sources: [
            path.resolve(__dirname, './fixtures/dc-consensus-source-1.geojson'),
            path.resolve(__dirname, './fixtures/dc-consensus-source-2.geojson'),
            path.resolve(__dirname, './fixtures/dc-consensus-source-3.geojson')
        ],
        'test_set': path.resolve(__dirname, './fixtures/dc-consensus-test-set.geojson'),
        'error_sources': '/tmp/error-sources',
        'error_test_set': '/tmp/error-test-set',
        context: {
            country: 'us',
            region: 'dc',
            languages: ['en']
        },
        db: 'pt_test'
    });

    t.deepEqual(results, {
        'source-3': { agreement_count: 0, hit_count: 1 },
        'source-2': { agreement_count: 1, hit_count: 1 },
        'source-1': { agreement_count: 1, hit_count: 1 }
    });

    t.doesNotThrow(() => {
        fs.accessSync('/tmp/error-sources');
        fs.accessSync('/tmp/error-test-set');
    });

    fs.unlinkSync('/tmp/error-sources');
    fs.unlinkSync('/tmp/error-test-set');
    t.end();
});
