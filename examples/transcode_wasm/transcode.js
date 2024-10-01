let wasmInstance;
let wasmMemory;
let wasmFunctions;
let transcoder;

(async () => {
    wasmInstance = (await WebAssembly.instantiateStreaming(fetch("transcode_wasm.wasm"))).instance;
    wasmMemory = wasmInstance.exports.memory;
    wasmFunctions = wasmInstance.exports;
})();

async function startTranscode() {
    const input = document.getElementById("input");

    const files = input.files;
    if (files === null || files.length === 0) {
        return;
    }
    const file = files[0];

    if (transcoder !== undefined) {
        wasmFunctions.freeTranscoder(transcoder);
    }
    const transcodeOptions = {};
    transcoder = wasmFunctions.newTranscoder(valueToWasmJson(transcodeOptions));

    // TODO: 所要時間を取る

    let resultWasmJson;
    let result;
    const inputBytes = new Uint8Array(await file.arrayBuffer());
    const inputWasmBytes = convertToWasmBytes(inputBytes);
    resultWasmJson = wasmFunctions.parseInputMp4File(transcoder, inputWasmBytes);
    result = wasmJsonToValue(resultWasmJson);
    if (result["Err"] !== undefined) {
        throw JSON.stringify(result);
    }
    console.log("Parsed: " + JSON.stringify(result));

    resultWasmJson = wasmFunctions.startTranscode(transcoder);
    result = wasmJsonToValue(resultWasmJson);
    if (result["Err"] !== undefined) {
        throw JSON.stringify(result);
    }
    console.log("startTranscode: " + JSON.stringify(result));
}

function convertToWasmBytes(bytes) {
    const wasmBytes = wasmFunctions.allocateVec(bytes.length);
    const wasmBytesOffset = wasmFunctions.vecOffset(wasmBytes);
    new Uint8Array(wasmMemory.buffer, wasmBytesOffset, bytes.length).set(bytes);
    return wasmBytes;
}

function valueToWasmJson(value) {
    const jsonBytes = new TextEncoder().encode(JSON.stringify(value));
    return convertToWasmBytes(jsonBytes);
}

function wasmJsonToValue(wasmJson) {
    const offset = wasmFunctions.vecOffset(wasmJson);
    const len = wasmFunctions.vecLen(wasmJson);
    const buffer = new Uint8Array(wasmMemory.buffer, offset, len);
    const value = JSON.parse(new TextDecoder("utf-8").decode(buffer));
    wasmFunctions.freeVec(wasmJson);
    return value;
}
