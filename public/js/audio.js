let audioContext;
let scriptProcessor;
let mediaStreamSource;
let localStream;
let isMuted = false;

// This function will be called from main.js to update the mute state
function setMute(shouldMute) {
    isMuted = shouldMute;
}

// Plays a raw audio chunk received from the WebSocket
async function playAudioChunk(audioData) {
    if (!audioContext) {
        // Lazy init AudioContext on first received chunk
        audioContext = new (window.AudioContext || window.webkitAudioContext)();
    }
    // The incoming data is a Float32Array, so we need to create a buffer of the same size
    const audioBuffer = audioContext.createBuffer(1, audioData.length, audioContext.sampleRate);
    audioBuffer.getChannelData(0).set(audioData);

    const source = audioContext.createBufferSource();
    source.buffer = audioBuffer;
    source.connect(audioContext.destination);
    source.start();
}

// Starts capturing audio from the microphone and sending it over the WebSocket
async function startAudioCapture(ws) {
    if (!ws || ws.readyState !== WebSocket.OPEN) {
        console.error("WebSocket is not open. Cannot start audio capture.");
        return;
    }

    try {
        localStream = await navigator.mediaDevices.getUserMedia({ audio: true });
        
        audioContext = new (window.AudioContext || window.webkitAudioContext)();
        // Use a buffer size of 4096 for better performance
        scriptProcessor = audioContext.createScriptProcessor(4096, 1, 1);
        mediaStreamSource = audioContext.createMediaStreamSource(localStream);

        scriptProcessor.onaudioprocess = (event) => {
            if (isMuted) {
                return; // If muted, don't send anything
            }
            const inputData = event.inputBuffer.getChannelData(0);
            // Send the raw Float32Array data buffer
            ws.send(inputData.buffer);
        };

        mediaStreamSource.connect(scriptProcessor);
        // It's important to connect the processor to the destination to keep it alive
        scriptProcessor.connect(audioContext.destination);

        console.log("Audio capture started successfully.");
        return true; // Indicate success

    } catch (err) {
        console.error("Error capturing audio:", err);
        alert("Could not get microphone access. Please check permissions.");
        return false; // Indicate failure
    }
}

function stopAudioCapture() {
    if (localStream) {
        localStream.getTracks().forEach(track => track.stop());
    }
    if (scriptProcessor) {
        scriptProcessor.disconnect();
    }
    if (mediaStreamSource) {
        mediaStreamSource.disconnect();
    }
    console.log("Audio capture stopped.");
}

export { startAudioCapture, stopAudioCapture, playAudioChunk, setMute };
