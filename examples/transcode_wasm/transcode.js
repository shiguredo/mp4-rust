let wasmInstance;
let wasmMemory;
let transcoder;

(async () => {
    wasmInstance = (await WebAssembly.instantiateStreaming(fetch("transcode_wasm.wasm"))).instance;
    wasmMemory = wasmInstance.exports.memory;
})();

async function startTranscode() {
    const input = document.getElementById("input");

    const files = input.files;
    if (files === null || files.length === 0) {
        return;
    }
    const file = files[0];

    const inputBytes = new Uint8Array(await file.arrayBuffer());
    const buffer = wasmInstance.exports.allocateVec(inputBytes.length);
    const bufferOffset = wasmInstance.exports.vecOffset(buffer);
    new Uint8Array(wasmMemory.buffer, bufferOffset, inputBytes.length).set(inputBytes);

    // const output = wasmInstance.exports.transcode(bufferOffset, inputBytes.length);
    // const outputOffset = wasmInstance.exports.vec_offset(output);
    // const outputLen = wasmInstance.exports.vec_len(output);
    // const outputText = new TextDecoder('utf-8').decode(
    //     new Uint8Array(wasmMemory.buffer, outputOffset, outputLen));
    // wasmInstance.exports.free_vec(output);
    // document.getElementById("output").value = outputText;
    alert("hello");
}
