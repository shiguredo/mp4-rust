let wasmInstance;
let wasmMemory;
let wasmFunctions;
let transcoder;
let nextCoderId = 0;
let coders = {};
let coderErrors = {};
let coderResultFutures = {};
let logMessages = [];
let lastLogTime;
let lastTimeoutId;

(async () => {
    const importObject = {
        env: {
            consoleLog(messageOffset, messageLen) {
                console.log(new TextDecoder('utf-8').decode(
                    new Uint8Array(wasmMemory.buffer,messageOffset, messageLen)));
            },
            async closeCoder(coderId) {
                coders[coderId].close();
            },
            async createVideoDecoder(resultFuture, configWasmJson) {
                const config = wasmJsonToValue(configWasmJson);
                config.description = new Uint8Array(config.description);

                const coderId = nextCoderId;

                const params = {
                    output: function (frame) {
                        let future = coderResultFutures[coderId];
                        let result = {"Ok": {
                            width: frame.codedWidth,
                            height: frame.codedHeight,
                        }};
                        let size = frame.allocationSize({format: "RGBA"});
                        let wasmBytes = wasmFunctions.allocateVec(size);
                        let wasmBytesOffset = wasmFunctions.vecOffset(wasmBytes);
                        frame.copyTo(new Uint8Array(wasmMemory.buffer, wasmBytesOffset, size), {format: "RGBA"});
                        frame.close();
                        wasmFunctions.notifyDecodeSampleResult(
                            transcoder, future, valueToWasmJson(result), wasmBytes);
                    },
                    error: function (error) {
                        if (coderResultFutures[coderId] !== undefined) {
                            // サンプルデコード中のエラー
                            const result = {"Err": {"message": String(error)}};
                            wasmFunctions.notifyDecodeSampleResult(
                                transcoder, future, valueToWasmJson(result), wasmBytes);
                        } else {
                            // デコーダー初期化時のエラー
                            coderErrors[coderId] = String(error);
                        }
                    }
                };

                const decoder = new VideoDecoder(params);
                nextCoderId += 1;
                coders[coderId] = decoder;

                // 不正な config を指定したとしても、この呼び出しは常に成功する
                await decoder.configure(config);

                let result = {"Ok": coderId};
                wasmInstance.exports.notifyCreateVideoDecoderResult(
                    transcoder, resultFuture, valueToWasmJson(result));
            },
            async decodeSample(resultFuture, coderId, isKey, dataBytes, dataBytesLen) {
                if (coderErrors[coderId] !== undefined) {
                    result = {"Err": {"message": coderErrors[coderId]}};
                    wasmFunctions.notifyDecodeSampleResult(
                        transcoder, resultFuture, valueToWasmJson(result), null);
                    return;
                }
                if (coders[coderId] === undefined) {
                    // ここには来ないはず
                    throw "unknown coder id";
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
            async createVideoEncoder(resultFuture, configWasmJson) {
                const config = wasmJsonToValue(configWasmJson);
                const coderId = nextCoderId;

                const params = {
                    output: function (chunk) {
                        let future = coderResultFutures[coderId];
                        let result = {"Ok": null};
                        let size = chunk.byteLength;
                        let wasmBytes = wasmFunctions.allocateVec(size);
                        let wasmBytesOffset = wasmFunctions.vecOffset(wasmBytes);
                        chunk.copyTo(new Uint8Array(wasmMemory.buffer, wasmBytesOffset, size));
                        wasmFunctions.notifyEncodeSampleResult(
                            transcoder, future, valueToWasmJson(result), wasmBytes);
                    },
                    error: function (error) {
                        if (coderResultFutures[coderId] !== undefined) {
                            // サンプルエンコード中のエラー
                            const result = {"Err": {"message": String(error)}};
                            wasmFunctions.notifyEncodeSampleResult(
                                transcoder, future, valueToWasmJson(result), wasmBytes);
                        } else {
                            // エンコーダー初期化時のエラー
                            coderErrors[coderId] = String(error);
                        }
                    }
                };

                const encoder = new VideoEncoder(params);
                nextCoderId += 1;
                coders[coderId] = encoder;

                // 不正な config を指定したとしても、この呼び出しは常に成功する
                await encoder.configure(config);

                let result = {"Ok": coderId};
                wasmFunctions.notifyCreateVideoEncoderResult(transcoder, resultFuture, valueToWasmJson(result));
            },
            async encodeSample(resultFuture, coderId, isKey, width, height, dataBytes, dataBytesLen) {
                if (coderErrors[coderId] !== undefined) {
                    result = {"Err": {"message": coderErrors[coderId]}};
                    wasmFunctions.notifyEncodeSampleResult(
                        transcoder, resultFuture, valueToWasmJson(result), null);
                    return;
                }
                if (coders[coderId] === undefined) {
                    // ここには来ないはず
                    throw "unknown coder id";
                }

                const data = new Uint8Array(wasmMemory.buffer, dataBytes, dataBytesLen).slice();
                const encoder = coders[coderId];
                const frame = new VideoFrame(
                    data,
                    {
                        format: "RGBA",
                        codedWidth: width,
                        codedHeight: height,
                        timestamp: 0, // dummy value
                        duration: 0, // dummy value
                    });
                encoder.encode(frame, {keyFrame: isKey === 1});
                frame.close();
                coderResultFutures[coderId] = resultFuture;
            },
        }
    };
    wasmInstance = (await WebAssembly.instantiateStreaming(fetch("transcode_wasm.wasm"), importObject)).instance;
    wasmMemory = wasmInstance.exports.memory;
    wasmFunctions = wasmInstance.exports;
})();

async function startTranscode() {
    // 前回の状態をクリアする
    document.getElementById("output").classList.add('disabled-link');
    URL.revokeObjectURL(document.getElementById("output"));
    logMessages = [];
    if (transcoder !== undefined) {
        wasmFunctions.freeTranscoder(transcoder);
    }
    if (lastTimeoutId !== undefined) {
        clearTimeout(lastTimeoutId);
    }

    // 新規変換を始める
    const input = document.getElementById("input");

    const files = input.files;
    if (files === null || files.length === 0) {
        return;
    }
    const file = files[0];

    const resolution = document.getElementById('resolution').value;
    const [width, height] = resolution.split("x");
    const transcodeOptions = {
        codec: document.getElementById('codec').value,
        bitrate: Number(document.getElementById('bitrate').value),
        width: Number(width),
        height: Number(height),
        keyframeInterval: Number(document.getElementById('keyframeInterval').value),
    };
    transcoder = wasmFunctions.newTranscoder(valueToWasmJson(transcodeOptions));

    let resultWasmJson;
    let result;
    const inputBytes = new Uint8Array(await file.arrayBuffer());
    const inputWasmBytes = toWasmBytes(inputBytes);
    log(`Parsing input MP4 file ...`);
    resultWasmJson = wasmFunctions.parseInputMp4File(transcoder, inputWasmBytes);
    result = wasmJsonToValue(resultWasmJson);
    if (result["Err"] !== undefined) {
        logError(result);
        return;
    }
    logDone()
    log(`Input MP4 file size: ${Math.floor(inputBytes.byteLength / 1024 / 1024)} MB`);
    log(`Input MP4 file duration: ${result["Ok"]} seconds`);
    log("");

    resultWasmJson = wasmFunctions.startTranscode(transcoder);
    result = wasmJsonToValue(resultWasmJson);
    if (result["Err"] !== undefined) {
        logError(result);
        return;
    }
    log("Transcoding ...");

    pollTranscode();
}

function pollTranscode() {
    let resultWasmJson = wasmFunctions.pollTranscode(transcoder);
    let result = wasmJsonToValue(resultWasmJson);
    if (result["Err"] !== undefined) {
        logError(result);
        return;
    }

    const progress = result["Ok"];
    updateLog(`Transcoding ... ${(progress.rate * 100).toFixed(3)} %`);
    if (!progress.done) {
        lastTimeoutId = setTimeout(pollTranscode, 1000);
        return;
    }
    logDone("Transcoding ...");
    log("");

    log("Building output MP4 file ...");
    resultWasmJson = wasmFunctions.buildOutputMp4File(transcoder);
    result = wasmJsonToValue(resultWasmJson);
    if (result["Err"] !== undefined) {
        logError(result);
        return;
    }
    logDone();

    const output = document.getElementById("output");
    output.classList.remove('disabled-link');
    const mp4 = getOutputMp4File();
    log(`Output MP4 file size: ${Math.floor(mp4.byteLength / 1024 / 1024)} MB`);
    const blob = new Blob([mp4], { type: 'video/mp4' });
    output.href = URL.createObjectURL(blob);
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

function log(message) {
    lastLogTime = performance.now();
    logMessages.push(message);
    document.getElementById("log").value = logMessages.join("\n");
}

function updateLog(message) {
    const elapsed = (performance.now() - lastLogTime) / 1000;
    logMessages.pop();
    logMessages.push(`${message} (elapsed ${elapsed.toFixed(3)} seconds)`);
    document.getElementById("log").value = logMessages.join("\n");
}

function logDone(message) {
    const elapsed = (performance.now() - lastLogTime) / 1000;
    if (message === undefined) {
        message = logMessages.pop();
    } else {
        logMessages.pop();
    }
    log(`${message} done (elapsed ${elapsed.toFixed(3)} seconds)`);
}

function logError(result) {
    let detail = result["Err"];
    const message = logMessages.pop();
    log(`${message} error`);
    log(JSON.stringify(detail, null, 2));
}
