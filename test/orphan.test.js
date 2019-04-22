'use strict';

const Orphan = require('../lib/map/orphan');
const {
    pg_optimize,
    cluster_addr
} = require('../native/index.node');
const test = require('tape');

const db = require('./lib/db');

db.init(test);

const fs = require('fs');
const path = require('path');
const Queue = require('d3-queue').queue;
const readline = require('readline');
const output = fs.createWriteStream(path.resolve(__dirname, '../test/fixtures/orphan-output.geojson'));

test('orphan.address', (t) => {
    const pool = db.get();

    const orphan = new Orphan(pool, {
        props: ['accuracy']
    }, output);
    const popQ = new Queue(1);

    // populate address
    popQ.defer((done) => {
        pool.query(`
            BEGIN;
            INSERT INTO address (names, number, geom, netid, props) VALUES ('[{ "tokenized": "main st se", "tokenless": "main", "display": "Main Street SE" }]', 1, ST_SetSRID(ST_GeomFromGeoJSON('{ "type": "Point", "coordinates": [-66.97265625,43.96119063892024] }'), 4326), 1, '{ "accuracy": "building" }');
            INSERT INTO address (names, number, geom, netid, props) VALUES ('[{ "tokenized": "main st se", "tokenless": "main", "display": "Main Street SE" }]', 2, ST_SetSRID(ST_GeomFromGeoJSON('{ "type": "Point", "coordinates": [-66.97265625,43.96119063892024] }'), 4326), 1, '{ "accuracy": "building" }');
            INSERT INTO address (names, number, geom, netid, props) VALUES ('[{ "tokenized": "main st", "tokenless": "main", "display": "Main Street" }]', 3, ST_SetSRID(ST_GeomFromGeoJSON('{ "type": "Point", "coordinates": [-105.46875,56.36525013685606] }'), 4326), NULL, '{ "accuracy": "parcel" }');
            INSERT INTO address (names, number, geom, netid, props) VALUES ('[{ "tokenized": "main st", "tokenless": "main", "display": "Main Street" }]', 4, ST_SetSRID(ST_GeomFromGeoJSON('{ "type": "Point", "coordinates": [-105.46875,56.36525013685606] }'), 4326), NULL, '{ "accuracy": "parcel" }');
            INSERT INTO address (names, number, geom, netid, props) VALUES ('[{ "tokenized": "fake av", "tokenless": "fake", "display": "Fake Avenue" }]', 5, ST_SetSRID(ST_GeomFromGeoJSON('{ "type": "Point", "coordinates": [-85.25390625,52.908902047770255] }'), 4326), NULL, '{ "accuracy": "building" }');
            INSERT INTO address (names, number, geom, netid, props) VALUES ('[{ "tokenized": "main st se", "tokenless": "main", "display": "Main Street SE" }]', 6, ST_SetSRID(ST_GeomFromGeoJSON('{ "type": "Point", "coordinates": [-66.97265625,43.96119063892024] }'), 4326), 1, '{ "accuracy": "parcel" }');
            COMMIT;
        `, (err) => {
            t.error(err, 'ok - added addresses to table');

            pg_optimize();

            return done();
        });
    });

    // call orphan.address
    popQ.defer((done) => {
        cluster_addr('pt_test', true);

        orphan.address((err) => {
            t.error(err);
            return done();
        });
    });

    // check address_orphan_cluster
    popQ.defer((done) => {
        pool.query(`
            SELECT names FROM address_orphan_cluster ORDER BY names;
        `, (err, res) => {
            t.error(err);

            t.equals(res.rows.length, 2, 'ok - correct number of orphans');
            t.deepEquals(res.rows[0], { names: [{ display: 'Fake Avenue', tokenized: 'fake av', tokenless: 'fake' }] }, 'ok - Fake Ave orphaned');
            t.deepEquals(res.rows[1], { names: [{ display: 'Main Street', tokenized: 'main st', tokenless: 'main' }] }, 'ok - Main St orphaned');
            return done();
        });
    });

    popQ.await((err) => {
        t.error(err);
        output.end();

        pool.end(() => {
            t.end();
        });
    });
});

db.init(test);

test('orphan output', (t) => {
    let counter = 0;
    const orphans = {
        'Main Street': [['3','4']],
        'Fake Avenue': [['5']]
    };

    const rl = readline.createInterface({
        input : fs.createReadStream(path.resolve(__dirname, '../test/fixtures/orphan-output.geojson'))
    });
    rl.on('line', (line) => {
        if (!line) return;
        counter++;
        const feat = JSON.parse(line);

        t.deepEquals(feat.properties['carmen:addressnumber'], orphans[feat.properties['carmen:text']], 'ok - orphan has correct addresses');

        t.ok(feat.properties.accuracy);
    });

    rl.on('close', () => {
        t.equals(counter, 2, 'ok - output had correct number of orphan clusters');
        t.end();
    });
});

db.init(test);
