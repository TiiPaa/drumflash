import React, { useState, useEffect, useRef } from "react";
import {
  Play,
  Pause,
  Square,
  RotateCcw,
  Settings,
  Volume2,
  VolumeX,
  Sparkles,
  Download,
} from "lucide-react";

const DrumPatternGenerator = () => {
  const [isPlaying, setIsPlaying] = useState(false);
  const [bpm, setBpm] = useState(120);
  const [currentStep, setCurrentStep] = useState(0);
  const [showSoundMenu, setShowSoundMenu] = useState(false);
  const [isGenerating, setIsGenerating] = useState(false);
  const [randomBPM, setRandomBPM] = useState(false);
  const [randomSounds, setRandomSounds] = useState(false);
  const [midiOutput, setMidiOutput] = useState(null);
  const [midiOutputs, setMidiOutputs] = useState([]);
  const [midiEnabled, setMidiEnabled] = useState(false);

  const [patterns, setPatterns] = useState({
    BD: Array(16).fill(false),
    SD: Array(16).fill(false),
    HH: Array(16).fill(false),
    OH: Array(16).fill(false),
    T1: Array(16).fill(false),
    T2: Array(16).fill(false),
    T3: Array(16).fill(false),
  });

  const [soundSettings, setSoundSettings] = useState({
    BD: { frequency: 60, decay: 0.5, volume: 0.8, filter: 100 },
    SD: { frequency: 200, decay: 0.2, volume: 0.6, filter: 1000 },
    HH: { frequency: 8000, decay: 0.1, volume: 0.3, filter: 10000 },
    OH: { frequency: 6000, decay: 0.3, volume: 0.4, filter: 8000 },
    T1: { frequency: 300, decay: 0.3, volume: 0.5, filter: 2000 },
    T2: { frequency: 200, decay: 0.4, volume: 0.5, filter: 1500 },
    T3: { frequency: 120, decay: 0.5, volume: 0.5, filter: 1000 },
  });

  const [mutedInstruments, setMutedInstruments] = useState({
    BD: false,
    SD: false,
    HH: false,
    OH: false,
    T1: false,
    T2: false,
    T3: false,
  });

  const intervalRef = useRef(null);
  const audioContextRef = useRef(null);

  useEffect(() => {
    audioContextRef.current = new (window.AudioContext ||
      window.webkitAudioContext)();

    // Initialiser MIDI avec gestion des restrictions
    console.log("=== INITIALISATION MIDI ===");

    if (navigator.requestMIDIAccess) {
      console.log("Web MIDI API disponible");
      navigator
        .requestMIDIAccess({ sysex: false })
        .then(onMIDISuccess)
        .catch(onMIDIFailure);
    } else {
      console.log("Web MIDI API non disponible");
      setMidiOutputs([]);
    }

    return () => {
      if (audioContextRef.current) {
        audioContextRef.current.close();
      }
    };
  }, []);

  const onMIDISuccess = (midiAccess) => {
    console.log("=== MIDI CONNECTÉ AVEC SUCCÈS ===");
    const outputs = [];

    midiAccess.outputs.forEach((output, key) => {
      console.log(`Sortie MIDI trouvée - ID: ${key}, Nom: ${output.name}`);
      outputs.push({
        id: key,
        name: output.name || `Port MIDI ${key}`,
        output: output,
        state: output.state,
      });
    });

    console.log("Total sorties MIDI trouvées:", outputs.length);
    setMidiOutputs(outputs);

    if (outputs.length > 0) {
      setMidiOutput(outputs[0].output);
    }

    midiAccess.onstatechange = (event) => {
      console.log("Changement d'état MIDI:", event.port.name, event.port.state);
      onMIDISuccess(midiAccess);
    };
  };

  const onMIDIFailure = (error) => {
    console.log("=== ERREUR MIDI ===");
    console.error("Erreur détaillée:", error);

    // Détecter si c'est une erreur de permissions
    const isPermissionError =
      error.name === "SecurityError" ||
      error.message.includes("permissions policy") ||
      error.message.includes("Midi has been disabled");

    if (isPermissionError) {
      console.log("Erreur de permissions MIDI détectée");
      setMidiOutputs("permission_error");
    } else {
      setMidiOutputs([]);
    }
  };

  const sendMIDINote = (note, velocity = 100, duration = 100) => {
    if (midiOutput && midiEnabled) {
      midiOutput.send([0x99, note, velocity]);
      setTimeout(() => {
        midiOutput.send([0x89, note, 0]);
      }, duration);
    }
  };

  useEffect(() => {
    if (isPlaying) {
      const stepDuration = (60 / bpm / 4) * 1000;
      intervalRef.current = setInterval(() => {
        setCurrentStep((prev) => {
          const nextStep = (prev + 1) % 16;
          playStep(prev);
          return nextStep;
        });
      }, stepDuration);
    } else {
      clearInterval(intervalRef.current);
    }

    return () => clearInterval(intervalRef.current);
  }, [isPlaying, bpm, patterns, mutedInstruments]);

  const createDrumSound = (type) => {
    const ctx = audioContextRef.current;
    if (!ctx) return;

    const settings = soundSettings[type];

    switch (type) {
      case "BD":
        {
          const osc = ctx.createOscillator();
          const gain = ctx.createGain();
          const filter = ctx.createBiquadFilter();

          osc.connect(filter);
          filter.connect(gain);
          gain.connect(ctx.destination);

          const now = ctx.currentTime;
          osc.type = "sine";
          osc.frequency.setValueAtTime(settings.frequency, now);
          osc.frequency.exponentialRampToValueAtTime(
            settings.frequency * 0.1,
            now + 0.1
          );

          filter.type = "lowpass";
          filter.frequency.setValueAtTime(settings.filter, now);

          gain.gain.setValueAtTime(settings.volume, now);
          gain.gain.exponentialRampToValueAtTime(0.01, now + settings.decay);

          osc.start(now);
          osc.stop(now + settings.decay);
        }
        break;

      case "SD":
        {
          const osc = ctx.createOscillator();
          const gain = ctx.createGain();
          const filter = ctx.createBiquadFilter();

          osc.connect(filter);
          filter.connect(gain);
          gain.connect(ctx.destination);

          const now = ctx.currentTime;
          osc.type = "triangle";
          osc.frequency.setValueAtTime(settings.frequency, now);

          filter.type = "highpass";
          filter.frequency.setValueAtTime(settings.filter, now);

          gain.gain.setValueAtTime(settings.volume, now);
          gain.gain.exponentialRampToValueAtTime(0.01, now + settings.decay);

          osc.start(now);
          osc.stop(now + settings.decay);
        }
        break;

      case "HH":
        {
          const bufferSize = ctx.sampleRate * settings.decay;
          const buffer = ctx.createBuffer(1, bufferSize, ctx.sampleRate);
          const data = buffer.getChannelData(0);

          for (let i = 0; i < bufferSize; i++) {
            data[i] = (Math.random() * 2 - 1) * Math.pow(1 - i / bufferSize, 4);
          }

          const noise = ctx.createBufferSource();
          const gain = ctx.createGain();
          const filter = ctx.createBiquadFilter();

          noise.buffer = buffer;
          noise.connect(filter);
          filter.connect(gain);
          gain.connect(ctx.destination);

          const now = ctx.currentTime;
          filter.type = "highpass";
          filter.frequency.setValueAtTime(settings.filter, now);

          gain.gain.setValueAtTime(settings.volume, now);
          gain.gain.exponentialRampToValueAtTime(0.01, now + settings.decay);

          noise.start(now);
        }
        break;

      case "OH":
        {
          const bufferSize = ctx.sampleRate * settings.decay;
          const buffer = ctx.createBuffer(1, bufferSize, ctx.sampleRate);
          const data = buffer.getChannelData(0);

          for (let i = 0; i < bufferSize; i++) {
            data[i] =
              (Math.random() * 2 - 1) * Math.pow(1 - i / bufferSize, 1.5);
          }

          const noise = ctx.createBufferSource();
          const gain = ctx.createGain();
          const filter = ctx.createBiquadFilter();

          noise.buffer = buffer;
          noise.connect(filter);
          filter.connect(gain);
          gain.connect(ctx.destination);

          const now = ctx.currentTime;
          filter.type = "bandpass";
          filter.frequency.setValueAtTime(settings.filter, now);

          gain.gain.setValueAtTime(settings.volume, now);
          gain.gain.exponentialRampToValueAtTime(0.01, now + settings.decay);

          noise.start(now);
        }
        break;

      default:
        {
          const osc = ctx.createOscillator();
          const gain = ctx.createGain();
          const filter = ctx.createBiquadFilter();

          osc.connect(filter);
          filter.connect(gain);
          gain.connect(ctx.destination);

          const now = ctx.currentTime;
          osc.type = "sine";
          osc.frequency.setValueAtTime(settings.frequency, now);
          osc.frequency.exponentialRampToValueAtTime(
            settings.frequency * 0.3,
            now + 0.1
          );

          filter.type = "bandpass";
          filter.frequency.setValueAtTime(settings.filter, now);

          gain.gain.setValueAtTime(settings.volume, now);
          gain.gain.exponentialRampToValueAtTime(0.01, now + settings.decay);

          osc.start(now);
          osc.stop(now + settings.decay);
        }
        break;
    }
  };

  const playStep = (step) => {
    const drumMIDIMap = {
      BD: 36,
      SD: 38,
      HH: 42,
      OH: 46,
      T1: 50,
      T2: 47,
      T3: 43,
    };

    Object.keys(patterns).forEach((drum) => {
      if (patterns[drum][step] && !mutedInstruments[drum]) {
        createDrumSound(drum);

        if (midiEnabled && midiOutput) {
          const midiNote = drumMIDIMap[drum];
          sendMIDINote(midiNote, 100, 100);
        }
      }
    });
  };

  const toggleStep = (drum, step) => {
    setPatterns((prev) => ({
      ...prev,
      [drum]: prev[drum].map((active, index) =>
        index === step ? !active : active
      ),
    }));
  };

  const toggleMute = (drum) => {
    setMutedInstruments((prev) => ({
      ...prev,
      [drum]: !prev[drum],
    }));
  };

  const clearPattern = () => {
    setPatterns((prev) =>
      Object.keys(prev).reduce(
        (acc, drum) => ({
          ...acc,
          [drum]: Array(16).fill(false),
        }),
        {}
      )
    );
  };

  const randomizeSounds = () => {
    setSoundSettings((prev) => {
      const newSettings = { ...prev };

      Object.keys(newSettings).forEach((drum) => {
        const baseSettings = {
          BD: {
            freqMin: 40,
            freqMax: 120,
            decayMin: 0.3,
            decayMax: 0.8,
            volMin: 0.6,
            volMax: 1.0,
            filterMult: 2,
          },
          SD: {
            freqMin: 150,
            freqMax: 300,
            decayMin: 0.1,
            decayMax: 0.4,
            volMin: 0.4,
            volMax: 0.8,
            filterMult: 8,
          },
          HH: {
            freqMin: 6000,
            freqMax: 12000,
            decayMin: 0.05,
            decayMax: 0.2,
            volMin: 0.2,
            volMax: 0.5,
            filterMult: 1.5,
          },
          OH: {
            freqMin: 4000,
            freqMax: 10000,
            decayMin: 0.2,
            decayMax: 0.6,
            volMin: 0.3,
            volMax: 0.6,
            filterMult: 1.2,
          },
          T1: {
            freqMin: 200,
            freqMax: 500,
            decayMin: 0.2,
            decayMax: 0.5,
            volMin: 0.3,
            volMax: 0.7,
            filterMult: 4,
          },
          T2: {
            freqMin: 150,
            freqMax: 350,
            decayMin: 0.3,
            decayMax: 0.6,
            volMin: 0.3,
            volMax: 0.7,
            filterMult: 3,
          },
          T3: {
            freqMin: 80,
            freqMax: 200,
            decayMin: 0.4,
            decayMax: 0.8,
            volMin: 0.3,
            volMax: 0.7,
            filterMult: 2,
          },
        };

        const base = baseSettings[drum];
        if (base) {
          newSettings[drum] = {
            frequency:
              Math.floor(Math.random() * (base.freqMax - base.freqMin + 1)) +
              base.freqMin,
            decay:
              Math.round(
                (Math.random() * (base.decayMax - base.decayMin) +
                  base.decayMin) *
                  10
              ) / 10,
            volume:
              Math.round(
                (Math.random() * (base.volMax - base.volMin) + base.volMin) * 10
              ) / 10,
            filter: Math.floor(newSettings[drum].frequency * base.filterMult),
          };
        }
      });

      return newSettings;
    });
  };

  const generateRandomPattern = () => {
    setPatterns((prev) =>
      Object.keys(prev).reduce(
        (acc, drum) => ({
          ...acc,
          [drum]: Array(16)
            .fill(false)
            .map(() => Math.random() < 0.3),
        }),
        {}
      )
    );

    if (randomBPM) {
      const randomBPMValue = Math.floor(Math.random() * (180 - 60 + 1)) + 60;
      setBpm(randomBPMValue);
    }

    if (randomSounds) {
      randomizeSounds();
    }
  };

  const generateAIPattern = async (style) => {
    setIsGenerating(true);

    const styleBPM = {
      rock: 120,
      techno: 128,
      rap: 90,
      jazz: 140,
      reggae: 75,
      metal: 160,
      funk: 110,
      latin: 100,
      disco: 120,
      trap: 140,
    };

    const prompt = `Generate a drum pattern for a ${style} style in JSON format.
The pattern should have 16 steps (beats) for each instrument.
Each instrument array should contain 16 boolean values (true = hit, false = silence).

Available instruments:
- BD: Bass Drum (kick)
- SD: Snare Drum
- HH: Hi-Hat (closed)
- OH: Open Hi-Hat
- T1: High Tom
- T2: Mid Tom
- T3: Low Tom

Create a pattern that fits the ${style} style characteristics.

Respond ONLY with valid JSON in this exact format:
{
  "BD": [true, false, false, false, true, false, false, false, true, false, false, false, true, false, false, false],
  "SD": [false, false, false, false, true, false, false, false, false, false, false, false, true, false, false, false],
  "HH": [true, false, true, false, true, false, true, false, true, false, true, false, true, false, true, false],
  "OH": [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
  "T1": [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
  "T2": [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
  "T3": [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false]
}

DO NOT include any text other than the JSON.`;

    try {
      const response = await window.claude.complete(prompt);
      const aiPattern = JSON.parse(response);

      const requiredInstruments = ["BD", "SD", "HH", "OH", "T1", "T2", "T3"];
      const isValid = requiredInstruments.every(
        (instrument) =>
          aiPattern[instrument] &&
          Array.isArray(aiPattern[instrument]) &&
          aiPattern[instrument].length === 16 &&
          aiPattern[instrument].every((step) => typeof step === "boolean")
      );

      if (isValid) {
        setPatterns(aiPattern);
        if (randomBPM) {
          const randomBPMValue =
            Math.floor(Math.random() * (180 - 60 + 1)) + 60;
          setBpm(randomBPMValue);
        } else if (styleBPM[style.toLowerCase()]) {
          setBpm(styleBPM[style.toLowerCase()]);
        }

        if (randomSounds) {
          randomizeSounds();
        }
      } else {
        console.error("Invalid AI pattern structure");
      }
    } catch (error) {
      console.error("Error generating AI pattern:", error);
    } finally {
      setIsGenerating(false);
    }
  };

  const exportToMIDI = () => {
    const drumMIDIMap = {
      BD: 36,
      SD: 38,
      HH: 42,
      OH: 46,
      T1: 50,
      T2: 47,
      T3: 43,
    };

    const createMIDIFile = () => {
      const ticksPerQuarter = 480;
      const ticksPerStep = ticksPerQuarter / 4;

      const header = new Uint8Array([
        0x4d, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01,
        0x01, 0xe0,
      ]);

      const events = [];
      const microsecondsPerQuarter = Math.round(60000000 / bpm);
      events.push({
        deltaTime: 0,
        data: [
          0xff,
          0x51,
          0x03,
          (microsecondsPerQuarter >> 16) & 0xff,
          (microsecondsPerQuarter >> 8) & 0xff,
          microsecondsPerQuarter & 0xff,
        ],
      });

      Object.keys(patterns).forEach((drum) => {
        const midiNote = drumMIDIMap[drum];
        patterns[drum].forEach((active, step) => {
          if (active) {
            const tickTime = step * ticksPerStep;
            events.push({
              deltaTime: tickTime,
              data: [0x99, midiNote, 100],
            });
            events.push({
              deltaTime: tickTime + 10,
              data: [0x89, midiNote, 0],
            });
          }
        });
      });

      events.sort((a, b) => a.deltaTime - b.deltaTime);
      let lastTime = 0;
      events.forEach((event) => {
        const currentTime = event.deltaTime;
        event.deltaTime = currentTime - lastTime;
        lastTime = currentTime;
      });

      events.push({ deltaTime: 0, data: [0xff, 0x2f, 0x00] });

      const encodeVariableLength = (value) => {
        if (value < 128) return [value];
        const bytes = [];
        let temp = value;
        while (temp > 0) {
          bytes.unshift((temp & 0x7f) | (bytes.length > 0 ? 0x80 : 0));
          temp >>= 7;
        }
        return bytes;
      };

      let trackData = [];
      events.forEach((event) => {
        trackData.push(...encodeVariableLength(event.deltaTime));
        trackData.push(...event.data);
      });

      const trackHeader = new Uint8Array([
        0x4d,
        0x54,
        0x72,
        0x6b,
        (trackData.length >> 24) & 0xff,
        (trackData.length >> 16) & 0xff,
        (trackData.length >> 8) & 0xff,
        trackData.length & 0xff,
      ]);

      const midiFile = new Uint8Array(
        header.length + trackHeader.length + trackData.length
      );
      midiFile.set(header, 0);
      midiFile.set(trackHeader, header.length);
      midiFile.set(trackData, header.length + trackHeader.length);

      return midiFile;
    };

    try {
      const midiData = createMIDIFile();
      const blob = new Blob([midiData], { type: "audio/midi" });
      const url = URL.createObjectURL(blob);

      const link = document.createElement("a");
      link.href = url;
      link.download = `drum_pattern_${bpm}bpm.mid`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
    } catch (error) {
      console.error("Erreur lors de l'export MIDI:", error);
    }
  };

  const handleDragStart = (e) => {
    const drumMIDIMap = {
      BD: 36,
      SD: 38,
      HH: 42,
      OH: 46,
      T1: 50,
      T2: 47,
      T3: 43,
    };
    const ticksPerQuarter = 480;
    const ticksPerStep = ticksPerQuarter / 4;

    const header = new Uint8Array([
      0x4d, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01,
      0x01, 0xe0,
    ]);

    const events = [];
    const microsecondsPerQuarter = Math.round(60000000 / bpm);
    events.push({
      deltaTime: 0,
      data: [
        0xff,
        0x51,
        0x03,
        (microsecondsPerQuarter >> 16) & 0xff,
        (microsecondsPerQuarter >> 8) & 0xff,
        microsecondsPerQuarter & 0xff,
      ],
    });

    Object.keys(patterns).forEach((drum) => {
      const midiNote = drumMIDIMap[drum];
      patterns[drum].forEach((active, step) => {
        if (active) {
          const tickTime = step * ticksPerStep;
          events.push({ deltaTime: tickTime, data: [0x99, midiNote, 100] });
          events.push({ deltaTime: tickTime + 10, data: [0x89, midiNote, 0] });
        }
      });
    });

    events.sort((a, b) => a.deltaTime - b.deltaTime);
    let lastTime = 0;
    events.forEach((event) => {
      const currentTime = event.deltaTime;
      event.deltaTime = currentTime - lastTime;
      lastTime = currentTime;
    });

    events.push({ deltaTime: 0, data: [0xff, 0x2f, 0x00] });

    const encodeVariableLength = (value) => {
      if (value < 128) return [value];
      const bytes = [];
      let temp = value;
      while (temp > 0) {
        bytes.unshift((temp & 0x7f) | (bytes.length > 0 ? 0x80 : 0));
        temp >>= 7;
      }
      return bytes;
    };

    let trackData = [];
    events.forEach((event) => {
      trackData.push(...encodeVariableLength(event.deltaTime));
      trackData.push(...event.data);
    });

    const trackHeader = new Uint8Array([
      0x4d,
      0x54,
      0x72,
      0x6b,
      (trackData.length >> 24) & 0xff,
      (trackData.length >> 16) & 0xff,
      (trackData.length >> 8) & 0xff,
      trackData.length & 0xff,
    ]);

    const midiFile = new Uint8Array(
      header.length + trackHeader.length + trackData.length
    );
    midiFile.set(header, 0);
    midiFile.set(trackHeader, header.length);
    midiFile.set(trackData, header.length + trackHeader.length);

    const blob = new Blob([midiFile], { type: "audio/midi" });
    const file = new File([blob], `drum_pattern_${bpm}bpm.mid`, {
      type: "audio/midi",
    });
    e.dataTransfer.setData(
      "DownloadURL",
      `audio/midi:drum_pattern_${bpm}bpm.mid:${URL.createObjectURL(blob)}`
    );
    e.dataTransfer.effectAllowed = "copy";
  };

  const updateSoundSetting = (drum, parameter, value) => {
    setSoundSettings((prev) => ({
      ...prev,
      [drum]: {
        ...prev[drum],
        [parameter]: parseFloat(value),
      },
    }));
  };

  const testSound = (drum) => {
    if (!mutedInstruments[drum]) {
      createDrumSound(drum);

      if (midiEnabled && midiOutput) {
        const drumMIDIMap = {
          BD: 36,
          SD: 38,
          HH: 42,
          OH: 46,
          T1: 50,
          T2: 47,
          T3: 43,
        };
        const midiNote = drumMIDIMap[drum];
        sendMIDINote(midiNote, 100, 200);
      }
    }
  };

  const resetSoundSettings = () => {
    setSoundSettings({
      BD: { frequency: 60, decay: 0.5, volume: 0.8, filter: 100 },
      SD: { frequency: 200, decay: 0.2, volume: 0.6, filter: 1000 },
      HH: { frequency: 8000, decay: 0.1, volume: 0.3, filter: 10000 },
      OH: { frequency: 6000, decay: 0.3, volume: 0.4, filter: 8000 },
      T1: { frequency: 300, decay: 0.3, volume: 0.5, filter: 2000 },
      T2: { frequency: 200, decay: 0.4, volume: 0.5, filter: 1500 },
      T3: { frequency: 120, decay: 0.5, volume: 0.5, filter: 1000 },
    });
  };

  const presetPatterns = {
    "Rock Basique": {
      BD: [
        true,
        false,
        false,
        false,
        true,
        false,
        false,
        false,
        true,
        false,
        false,
        false,
        true,
        false,
        false,
        false,
      ],
      SD: [
        false,
        false,
        false,
        false,
        true,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        true,
        false,
        false,
        false,
      ],
      HH: [
        true,
        false,
        true,
        false,
        true,
        false,
        true,
        false,
        true,
        false,
        true,
        false,
        true,
        false,
        true,
        false,
      ],
      OH: Array(16).fill(false),
      T1: Array(16).fill(false),
      T2: Array(16).fill(false),
      T3: Array(16).fill(false),
    },
    "Funk Groove": {
      BD: [
        true,
        false,
        false,
        true,
        false,
        false,
        true,
        false,
        false,
        true,
        false,
        false,
        true,
        false,
        false,
        false,
      ],
      SD: [
        false,
        false,
        false,
        false,
        true,
        false,
        false,
        true,
        false,
        false,
        false,
        false,
        true,
        false,
        false,
        false,
      ],
      HH: [
        true,
        true,
        false,
        true,
        true,
        true,
        false,
        true,
        true,
        true,
        false,
        true,
        true,
        true,
        false,
        true,
      ],
      OH: [
        false,
        false,
        true,
        false,
        false,
        false,
        true,
        false,
        false,
        false,
        true,
        false,
        false,
        false,
        true,
        false,
      ],
      T1: Array(16).fill(false),
      T2: Array(16).fill(false),
      T3: Array(16).fill(false),
    },
    "Disco Beat": {
      BD: [
        true,
        false,
        false,
        false,
        true,
        false,
        false,
        false,
        true,
        false,
        false,
        false,
        true,
        false,
        false,
        false,
      ],
      SD: [
        false,
        false,
        false,
        false,
        true,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        true,
        false,
        false,
        false,
      ],
      HH: [
        true,
        true,
        true,
        true,
        false,
        true,
        true,
        true,
        true,
        true,
        true,
        true,
        false,
        true,
        true,
        true,
      ],
      OH: [
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
      ],
      T1: Array(16).fill(false),
      T2: Array(16).fill(false),
      T3: Array(16).fill(false),
    },
  };

  const loadPreset = (presetName) => {
    setPatterns(presetPatterns[presetName]);
  };

  const drumLabels = {
    BD: "Grosse Caisse",
    SD: "Caisse Claire",
    HH: "Charleston F.",
    OH: "Charleston O.",
    T1: "Tom Aigu",
    T2: "Tom Medium",
    T3: "Tom Grave",
  };

  const drumColors = {
    BD: "bg-red-500",
    SD: "bg-blue-500",
    HH: "bg-yellow-500",
    OH: "bg-orange-500",
    T1: "bg-green-500",
    T2: "bg-green-600",
    T3: "bg-green-700",
  };

  return (
    <div className="p-6 max-w-6xl mx-auto bg-gray-900 text-white rounded-lg">
      <h1 className="text-3xl font-bold mb-6 text-center">
        Générateur de Patterns de Boîte à Rythme
      </h1>

      {/* Contrôles principaux */}
      <div className="flex flex-wrap gap-4 mb-4 justify-center">
        <button
          onClick={() => setIsPlaying(!isPlaying)}
          className="flex items-center gap-2 px-4 py-2 bg-green-600 hover:bg-green-700 rounded-lg"
        >
          {isPlaying ? <Pause size={20} /> : <Play size={20} />}
          {isPlaying ? "Pause" : "Play"}
        </button>

        <button
          onClick={() => setIsPlaying(false)}
          className="flex items-center gap-2 px-4 py-2 bg-red-600 hover:bg-red-700 rounded-lg"
        >
          <Square size={20} />
          Stop
        </button>

        <button
          onClick={clearPattern}
          className="flex items-center gap-2 px-4 py-2 bg-gray-600 hover:bg-gray-700 rounded-lg"
        >
          <RotateCcw size={20} />
          Effacer
        </button>

        <button
          onClick={() => setShowSoundMenu(!showSoundMenu)}
          className="flex items-center gap-2 px-4 py-2 bg-indigo-600 hover:bg-indigo-700 rounded-lg"
        >
          <Settings size={20} />
          Sons
        </button>

        <button
          onClick={generateRandomPattern}
          className="px-4 py-2 bg-purple-600 hover:bg-purple-700 rounded-lg"
        >
          Aléatoire
        </button>

        <div className="flex items-center gap-2">
          <label>BPM:</label>
          <input
            type="range"
            min="60"
            max="180"
            value={bpm}
            onChange={(e) => setBpm(parseInt(e.target.value))}
            className="w-20"
          />
          <span className="w-12 text-center">{bpm}</span>
        </div>
      </div>

      {/* === SECTION MIDI === */}
      <div className="mb-6 bg-blue-900 border-2 border-blue-400 p-4 rounded-lg">
        <h2 className="text-xl font-bold mb-4 text-white text-center">
          🎹 SORTIE MIDI EN TEMPS RÉEL
        </h2>

        <div className="text-center mb-4">
          <label className="flex items-center justify-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              checked={midiEnabled}
              onChange={(e) => {
                console.log("MIDI toggle:", e.target.checked);
                setMidiEnabled(e.target.checked);
              }}
              className="w-6 h-6 text-green-600 bg-gray-800 border-2 border-white rounded focus:ring-green-500"
            />
            <span className="text-white text-lg font-bold">
              Activer sortie MIDI
            </span>
          </label>
        </div>

        {/* Menu MIDI - visible seulement si activé */}
        {midiEnabled && (
          <div className="bg-gray-800 p-4 rounded-lg space-y-4">
            <div className="text-center">
              <p className="text-white mb-2">
                Support MIDI :{" "}
                {navigator.requestMIDIAccess ? (
                  <span className="text-green-400">✅ Supporté</span>
                ) : (
                  <span className="text-red-400">❌ Non supporté</span>
                )}
              </p>
            </div>

            {/* Erreur de permissions */}
            {midiOutputs === "permission_error" ? (
              <div className="bg-red-900 border border-red-600 p-4 rounded-lg">
                <h4 className="text-red-300 font-bold mb-2 text-center">
                  🚫 MIDI Bloqué par Claude.ai
                </h4>
                <div className="text-red-200 text-sm space-y-2">
                  <p>
                    L'accès MIDI est bloqué par les politiques de sécurité de
                    Claude.ai.
                  </p>

                  <div className="bg-red-800 p-3 rounded mt-3">
                    <p className="font-semibold mb-2">
                      ✅ Solutions disponibles :
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-xs">
                      <li>
                        <strong>Export MIDI :</strong> Utilisez l'export MIDI
                        ci-dessous pour télécharger vos patterns
                      </li>
                      <li>
                        <strong>Drag & Drop :</strong> Glissez la zone d'export
                        directement dans votre DAW
                      </li>
                      <li>
                        <strong>Copier le code :</strong> Copiez ce code dans
                        votre propre serveur web
                      </li>
                    </ul>
                  </div>

                  <div className="bg-blue-800 p-3 rounded mt-3">
                    <p className="font-semibold mb-2">
                      🌐 Pour utiliser MIDI en temps réel :
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-xs">
                      <li>
                        Copiez ce code sur votre serveur local (localhost)
                      </li>
                      <li>
                        Ou utilisez un service comme CodePen, JSFiddle avec
                        HTTPS
                      </li>
                      <li>
                        Les navigateurs autorisent MIDI sur localhost et HTTPS
                      </li>
                    </ul>
                  </div>
                </div>
              </div>
            ) : Array.isArray(midiOutputs) && midiOutputs.length > 0 ? (
              <div>
                <p className="text-gray-300 text-center mb-3">
                  Périphériques détectés :{" "}
                  <span className="text-yellow-400 font-bold">
                    {midiOutputs.length}
                  </span>
                </p>

                <label className="block text-white font-medium mb-2 text-center">
                  Sélectionner le port MIDI :
                </label>
                <select
                  value={midiOutput?.id || ""}
                  onChange={(e) => {
                    console.log("Sélection port MIDI:", e.target.value);
                    const selectedOutput = midiOutputs.find(
                      (output) => output.output.id === e.target.value
                    );
                    if (selectedOutput) {
                      console.log("Port sélectionné:", selectedOutput.name);
                      setMidiOutput(selectedOutput.output);
                    } else {
                      setMidiOutput(null);
                    }
                  }}
                  className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded text-white text-center"
                >
                  <option value="">-- Choisir un port MIDI --</option>
                  {Array.isArray(midiOutputs) &&
                    midiOutputs.map((output) => (
                      <option key={output.id} value={output.output.id}>
                        {output.name}{" "}
                        {output.state === "connected" ? "✅" : "⚠️"}
                      </option>
                    ))}
                </select>

                {midiOutput && (
                  <div className="mt-3 text-center">
                    <div className="inline-block bg-green-600 text-white px-4 py-2 rounded-lg font-medium">
                      ✅ Connecté à : {midiOutput.name}
                    </div>
                  </div>
                )}

                <div className="mt-4 text-center">
                  <button
                    onClick={() => {
                      if (midiOutput) {
                        console.log("Test MIDI sur port:", midiOutput.name);
                        sendMIDINote(36, 100, 200);
                      }
                    }}
                    disabled={!midiOutput}
                    className="px-4 py-2 bg-yellow-600 hover:bg-yellow-700 disabled:bg-gray-600 disabled:cursor-not-allowed rounded text-white font-medium"
                  >
                    🎵 Test MIDI (Kick)
                  </button>
                </div>
              </div>
            ) : (
              <div className="text-center text-yellow-400">
                <p className="mb-2">⚠️ Recherche de périphériques MIDI...</p>
                <button
                  onClick={() => {
                    console.log("Relance de la détection MIDI...");
                    if (navigator.requestMIDIAccess) {
                      navigator
                        .requestMIDIAccess({ sysex: false })
                        .then(onMIDISuccess)
                        .catch(onMIDIFailure);
                    }
                  }}
                  className="mt-2 px-3 py-1 bg-blue-600 hover:bg-blue-700 rounded text-white text-sm"
                >
                  🔄 Relancer la détection
                </button>
              </div>
            )}

            {Array.isArray(midiOutputs) && midiOutputs.length > 0 && (
              <div className="border-t border-gray-600 pt-4">
                <h4 className="text-white font-medium mb-2 text-center">
                  💡 Comment utiliser :
                </h4>
                <ul className="text-sm text-gray-300 space-y-1">
                  <li>1. Sélectionnez votre port MIDI dans la liste</li>
                  <li>2. Testez la connexion avec le bouton "Test MIDI"</li>
                  <li>3. Lancez un pattern - les notes seront envoyées !</li>
                  <li>4. Canal MIDI utilisé : 10 (standard batterie)</li>
                </ul>
              </div>
            )}
          </div>
        )}

        {!midiEnabled && (
          <div className="text-center text-gray-400">
            <p>Cochez la case ci-dessus pour accéder aux options MIDI</p>
          </div>
        )}
      </div>

      {/* Options de randomisation */}
      <div className="mb-4 bg-gray-800 p-3 rounded-lg">
        <h3 className="text-sm font-semibold mb-2 text-gray-300">
          Options de génération :
        </h3>
        <div className="flex flex-wrap gap-4 justify-center">
          <label className="flex items-center gap-2 text-sm text-gray-300 cursor-pointer">
            <input
              type="checkbox"
              checked={randomBPM}
              onChange={(e) => setRandomBPM(e.target.checked)}
              className="w-4 h-4 rounded border-gray-600 bg-gray-700 text-purple-600 focus:ring-purple-500"
            />
            BPM aléatoire (60-180)
          </label>

          <label className="flex items-center gap-2 text-sm text-gray-300 cursor-pointer">
            <input
              type="checkbox"
              checked={randomSounds}
              onChange={(e) => setRandomSounds(e.target.checked)}
              className="w-4 h-4 rounded border-gray-600 bg-gray-700 text-purple-600 focus:ring-purple-500"
            />
            Sons aléatoires
          </label>

          <button
            onClick={randomizeSounds}
            className="px-3 py-1 bg-indigo-600 hover:bg-indigo-700 rounded text-sm"
          >
            Randomiser sons maintenant
          </button>
        </div>
      </div>

      {/* Export MIDI avec Drag & Drop */}
      <div className="mb-6 flex justify-center">
        <div className="bg-gradient-to-r from-emerald-600 to-teal-600 p-1 rounded-lg">
          <div
            draggable="true"
            onDragStart={handleDragStart}
            onClick={exportToMIDI}
            className="flex items-center gap-3 px-6 py-3 bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-800 transition-colors group"
          >
            <Download size={24} className="text-emerald-400" />
            <div className="text-left">
              <div className="text-white font-semibold">Export MIDI</div>
              <div className="text-gray-400 text-sm">
                Cliquez ou glissez dans votre DAW
              </div>
            </div>
            <div className="text-xs text-gray-500 ml-4 opacity-0 group-hover:opacity-100 transition-opacity">
              🎵 {bpm} BPM
            </div>
          </div>
        </div>
      </div>

      {/* Génération IA */}
      <div className="mb-6">
        <h3 className="text-lg font-semibold mb-3 text-center">
          Génération IA par Style
        </h3>
        <div className="flex flex-wrap gap-2 justify-center">
          <button
            onClick={() => generateAIPattern("rock")}
            disabled={isGenerating}
            className="flex items-center gap-2 px-3 py-2 bg-gradient-to-r from-pink-600 to-purple-600 hover:from-pink-700 hover:to-purple-700 rounded-lg disabled:opacity-50"
          >
            <Sparkles size={16} />
            {isGenerating ? "Génération..." : "AI Rock"}
          </button>

          <button
            onClick={() => generateAIPattern("techno")}
            disabled={isGenerating}
            className="flex items-center gap-2 px-3 py-2 bg-gradient-to-r from-blue-600 to-cyan-600 hover:from-blue-700 hover:to-cyan-700 rounded-lg disabled:opacity-50"
          >
            <Sparkles size={16} />
            {isGenerating ? "Génération..." : "AI Techno"}
          </button>

          <button
            onClick={() => generateAIPattern("rap")}
            disabled={isGenerating}
            className="flex items-center gap-2 px-3 py-2 bg-gradient-to-r from-orange-600 to-red-600 hover:from-orange-700 hover:to-red-700 rounded-lg disabled:opacity-50"
          >
            <Sparkles size={16} />
            {isGenerating ? "Génération..." : "AI Rap"}
          </button>
        </div>
      </div>

      {/* Menu de configuration des sons */}
      {showSoundMenu && (
        <div className="mb-6 bg-gray-800 p-4 rounded-lg">
          <div className="flex justify-between items-center mb-4">
            <h3 className="text-lg font-semibold">Configuration des Sons</h3>
            <button
              onClick={resetSoundSettings}
              className="px-3 py-1 bg-gray-600 hover:bg-gray-700 rounded text-sm"
            >
              Reset
            </button>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {Object.keys(soundSettings).map((drum) => (
              <div key={drum} className="bg-gray-700 p-3 rounded">
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-2">
                    <div
                      className={`w-4 h-4 rounded ${drumColors[drum]}`}
                    ></div>
                    <span className="font-medium">{drumLabels[drum]}</span>
                  </div>
                  <button
                    onClick={() => testSound(drum)}
                    className="flex items-center gap-1 px-2 py-1 bg-green-600 hover:bg-green-700 rounded text-xs"
                  >
                    <Volume2 size={12} />
                    Test
                  </button>
                </div>

                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <label className="text-xs text-gray-300">
                      Fréquence (Hz)
                    </label>
                    <span className="text-xs text-gray-400">
                      {soundSettings[drum].frequency}
                    </span>
                  </div>
                  <input
                    type="range"
                    min={
                      drum === "BD" || drum.startsWith("T")
                        ? "50"
                        : drum === "SD"
                        ? "100"
                        : "1000"
                    }
                    max={
                      drum === "BD"
                        ? "200"
                        : drum === "SD"
                        ? "500"
                        : drum.startsWith("T")
                        ? "800"
                        : "12000"
                    }
                    step="10"
                    value={soundSettings[drum].frequency}
                    onChange={(e) =>
                      updateSoundSetting(drum, "frequency", e.target.value)
                    }
                    className="w-full"
                  />

                  <div className="flex items-center justify-between">
                    <label className="text-xs text-gray-300">Volume</label>
                    <span className="text-xs text-gray-400">
                      {soundSettings[drum].volume.toFixed(1)}
                    </span>
                  </div>
                  <input
                    type="range"
                    min="0.1"
                    max="1"
                    step="0.1"
                    value={soundSettings[drum].volume}
                    onChange={(e) =>
                      updateSoundSetting(drum, "volume", e.target.value)
                    }
                    className="w-full"
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Presets */}
      <div className="mb-6">
        <h3 className="text-lg font-semibold mb-2">Presets:</h3>
        <div className="flex flex-wrap gap-2">
          {Object.keys(presetPatterns).map((preset) => (
            <button
              key={preset}
              onClick={() => loadPreset(preset)}
              className="px-3 py-1 bg-indigo-600 hover:bg-indigo-700 rounded text-sm"
            >
              {preset}
            </button>
          ))}
        </div>
      </div>

      {/* Pattern Grid */}
      <div className="bg-gray-800 p-4 rounded-lg overflow-x-auto">
        <div className="flex items-center mb-4">
          <div className="w-32 text-sm font-medium text-gray-400 mr-4">
            Instrument
          </div>
          <div className="flex gap-2">
            {Array.from({ length: 16 }, (_, i) => (
              <div key={i} className="w-10 text-center text-xs text-gray-400">
                {i + 1}
              </div>
            ))}
          </div>
        </div>

        {Object.keys(patterns).map((drum) => (
          <div key={drum} className="flex items-center mb-3">
            <div className="w-32 text-sm font-medium mr-4 flex items-center">
              <div
                className={`w-4 h-4 rounded ${drumColors[drum]} mr-2 ${
                  mutedInstruments[drum] ? "opacity-50" : ""
                }`}
              ></div>
              <span className={mutedInstruments[drum] ? "opacity-50" : ""}>
                {drumLabels[drum]}
              </span>
            </div>
            <button
              onClick={() => toggleMute(drum)}
              className={`mr-2 p-1 rounded ${
                mutedInstruments[drum]
                  ? "bg-red-600 hover:bg-red-700"
                  : "bg-gray-600 hover:bg-gray-700"
              }`}
              title={mutedInstruments[drum] ? "Unmute" : "Mute"}
            >
              {mutedInstruments[drum] ? (
                <VolumeX size={16} />
              ) : (
                <Volume2 size={16} />
              )}
            </button>
            <div className="flex gap-2">
              {patterns[drum].map((active, step) => (
                <button
                  key={step}
                  onClick={() => toggleStep(drum, step)}
                  className={`
                    w-10 h-10 rounded border-2 transition-all
                    ${
                      active
                        ? `${drumColors[drum]} border-white shadow-lg ${
                            mutedInstruments[drum] ? "opacity-50" : ""
                          }`
                        : "bg-gray-700 border-gray-600 hover:border-gray-500 hover:bg-gray-600"
                    }
                    ${
                      currentStep === step && isPlaying
                        ? "ring-2 ring-yellow-400 ring-offset-2 ring-offset-gray-800"
                        : ""
                    }
                  `}
                >
                  {active && (
                    <div className="w-full h-full flex items-center justify-center">
                      <div className="w-2 h-2 bg-white rounded-full"></div>
                    </div>
                  )}
                </button>
              ))}
            </div>
          </div>
        ))}
      </div>

      {/* Indicateur de lecture */}
      <div className="mt-4 text-center">
        <div className="text-sm text-gray-400">
          Step: {currentStep + 1} / 16
        </div>
        <div className="w-full bg-gray-700 rounded-full h-2 mt-2">
          <div
            className="bg-green-500 h-2 rounded-full transition-all duration-100"
            style={{ width: `${((currentStep + 1) / 16) * 100}%` }}
          />
        </div>
      </div>

      {/* Instructions */}
      <div className="mt-6 text-sm text-gray-400">
        <p>
          <strong>Instructions:</strong>
        </p>
        <p>• Cliquez sur les cases pour activer/désactiver les sons</p>
        <p>
          • <strong>MIDI :</strong> Cochez la section jaune en haut pour jouer
          sur vos instruments MIDI
        </p>
        <p>
          • Utilisez l'IA pour générer des patterns authentiques selon le style
          musical
        </p>
        <p>• Utilisez les boutons mute pour isoler des instruments</p>
        <p>
          • <strong>Export MIDI :</strong> Cliquez pour télécharger ou glissez
          directement dans votre DAW
        </p>
        <p>
          • BD = Grosse Caisse, SD = Caisse Claire, HH/OH = Charleston, T1/T2/T3
          = Toms
        </p>
      </div>
    </div>
  );
};

export default DrumPatternGenerator;
