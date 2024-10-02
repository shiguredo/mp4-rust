let wasmInstance;
let wasmMemory;
let wasmFunctions;
let transcoder;
let nextCoderId = 0;
let coders = {};
let coderErrors = {};
let coderResultFutures = {};

(async () => {
    const importObject = {
        env: {
            consoleLog(messageOffset, messageLen) {
                console.log(new TextDecoder('utf-8').decode(
                    new Uint8Array(wasmMemory.buffer,messageOffset, messageLen)));
            },
            async createVideoDecoder(resultFuture, configWasmJson) {
                const config = wasmJsonToValue(configWasmJson);
                console.log("createVideoDecoder: " + JSON.stringify(config));
                config.description = new Uint8Array(config.description);

                const coderId = nextCoderId;

                const params = {
                    output: function (frame) {
                        // console.log("decoded: " + frame.format + ", " + frame.codedWidth + "x" + frame.codedHeight);
                        let future = coderResultFutures[coderId]; // TODO: 取り出したら削除する
                        let result = {"Ok": null};
                        let size = frame.allocationSize({format: "RGBA"});
                        let wasmBytes = wasmFunctions.allocateVec(size);
                        let wasmBytesOffset = wasmFunctions.vecOffset(wasmBytes);
                        frame.copyTo(new Uint8Array(wasmMemory.buffer, wasmBytesOffset, size), {format: "RGBA"});
                        frame.close();
                        wasmFunctions.notifyDecodeSampleResult(
                            transcoder, future, valueToWasmJson(result), wasmBytes);
                    },
                    error: function (error) {
                        console.log("video decode error: " +  error);
                        coderErrors[coderId] = error;
                    }
                };

                const decoder = new VideoDecoder(params);
                nextCoderId += 1;
                coders[coderId] = decoder;
                await decoder.configure(config);

                // 不正な config を指定したとしても、ここは常に成功する
                let result = {"Ok": coderId};
                console.log("createVideoDecoderResult: " + JSON.stringify(result));
                wasmInstance.exports.notifyCreateVideoDecoderResult(
                    transcoder, resultFuture, valueToWasmJson(result));
            },
            async decodeSample(resultFuture, coderId, isKey, dataBytes, dataBytesLen) {
                // console.log("decodeSample: isKey=" + isKey);
                if (coderErrors[coderId] !== undefined) {
                    result = {"Err": {"message": coderErrors[coderId]}};
                    wasmFunctons.notifyDecodeSampleResult(
                        transcoder, resultFuture, valueToWasmJson(result), null);
                    return;
                }
                if (coders[coderId] === undefined) {
                    result = {"Err": {"message": "unknown decoder"}};
                    wasmFunctons.notifyDecodeSampleResult(
                        transcoder, resultFuture, valueToWasmJson(result), null);
                    return;
                }

                const decoder = coders[coderId];
                const chunk = new EncodedVideoChunk({
                    type: isKey === 1 ? "key" : "delta",
                    timestamp: 0, // dummy value
                    duration: 0, // dummy value
                    data: new Uint8Array(wasmMemory.buffer, dataBytes, dataBytesLen).slice(),
                });
                decoder.decode(chunk);
                coderResultFutures[coderId] = resultFuture;
            },
        }
    };
    wasmInstance = (await WebAssembly.instantiateStreaming(fetch("transcode_wasm.wasm"), importObject)).instance;
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
    const inputWasmBytes = toWasmBytes(inputBytes);
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

    pollTranscode();
}

function pollTranscode() {
    let resultWasmJson = wasmFunctions.pollTranscode(transcoder);
    let result = wasmJsonToValue(resultWasmJson);
    if (result["Err"] !== undefined) {
        throw JSON.stringify(result);
    }
    console.log("pollTranscode: " + JSON.stringify(result));
    if (!result["Ok"].done) {
        return setTimeout(pollTranscode, 1000); // TODO: もっと短くする
    }

    resultWasmJson = wasmFunctions.buildOutputMp4File(transcoder);
    result = wasmJsonToValue(resultWasmJson);
    if (result["Err"] !== undefined) {
        throw JSON.stringify(result);
    }
    console.log("buildOutputMp4File: " + JSON.stringify(result));
}

function download() {
    const mp4 = getOutputMp4File();
    const blob = new Blob([mp4], { type: 'video/mp4' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'output.mp4';
    a.click();
    URL.revokeObjectURL(url);
}

function getOutputMp4File() {
    const outputMp4WasmBytesRef = wasmFunctions.getOutputMp4File(transcoder);
    return fromWasmBytesRef(outputMp4WasmBytesRef);
}

function toWasmBytes(bytes) {
    const wasmBytes = wasmFunctions.allocateVec(bytes.length);
    const wasmBytesOffset = wasmFunctions.vecOffset(wasmBytes);
    new Uint8Array(wasmMemory.buffer, wasmBytesOffset, bytes.length).set(bytes);
    return wasmBytes;
}

function fromWasmBytesRef(wasmBytes) {
    const offset = wasmFunctions.vecOffset(wasmBytes);
    const len = wasmFunctions.vecLen(wasmBytes);
    return new Uint8Array(wasmMemory.buffer, offset, len);
}

function valueToWasmJson(value) {
    const jsonBytes = new TextEncoder().encode(JSON.stringify(value));
    return toWasmBytes(jsonBytes);
}

function wasmJsonToValue(wasmJson) {
    const offset = wasmFunctions.vecOffset(wasmJson);
    const len = wasmFunctions.vecLen(wasmJson);
    const buffer = new Uint8Array(wasmMemory.buffer, offset, len);
    const value = JSON.parse(new TextDecoder("utf-8").decode(buffer));
    wasmFunctions.freeVec(wasmJson);
    return value;
}
