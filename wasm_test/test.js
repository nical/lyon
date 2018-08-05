
var fs = require('fs');

var wasm_bin = fs.readFileSync('../target/wasm32-unknown-unknown/debug/wasm_test.wasm');

WebAssembly.instantiate(wasm_bin, { env: { Math_hypot: Math.hypot } }).then(obj => {
    //console.log(Object.keys(obj.instance.exports));
    console.log(" -- run tests...");
    obj.instance.exports.run_tests();
    console.log(" -- ...done!");
}).catch(err => {
    console.error(err);
    process.exit(1);
});
