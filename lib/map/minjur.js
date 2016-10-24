/**
 * Exposes a map function to convert/filter osm/minjur geometries to pt2itp formatted ones
 *
 * @param {Object} feat     GeoJSON feature to convert/filter
 * @return {Object|false}   Converted GeoJSON feature or false if it cannot be converted.
 */

module.exports.map = function(feat) {
    //Skip points & Polygons
    if (feat.geometry.type !== 'LineString' && feat.geometry.type !== 'MultiLineString') return false;

    //Skip non-highways
    if (!feat.properties.highway) return false;

    var accepted = [
        'motorway',
        'trunk',
        'primary',
        'secondary',
        'tertiary',
        'residential',
        'unclassified',
        'living_street',
        'road'
    ];

    //Eliminate all highway types not on accepted list
    if (accepted.indexOf(feat.properties.highway) === -1) return false;

    names = [];

    function addName(name) {
        if (!name) return;

        //OSM uses ; to separate in a single value
        name = name.split(';');
        name.forEach(function(n) {
            if (n && n.length > 0) names.push(n);
        });
    }

    ['name', 'loc_name', 'alt_name'].forEach(function(key) {
        addName(feat.properties[key]);
    });

    if (!names.length) names = '';
    else if (names.length === 1) names = names[0]; //String for 1 value array for many

    return {
        type: 'Feature',
        properties: {
            street: names
        },
        geometry: feat.geometry
    };
}