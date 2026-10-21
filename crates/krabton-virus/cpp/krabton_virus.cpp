// The Virus emulation of gearmulator behind a small C interface for Krabton:
// a firmware read and its presets listed, a device booted from it, MIDI in,
// audio out at the host's rate, and its state (the patches, as Virus sysex)
// saved and restored. Nothing here throws across the interface: every C++
// exception becomes an error message or a quiet failure.
//
// Copyright (C) Krabton contributors. GPL-3.0-only, as the code it calls.

#include <algorithm>
#include <cstring>
#include <memory>
#include <string>
#include <vector>

#include "baseLib/filesystem.h"
#include "synthLib/plugin.h"
#include "virusLib/device.h"
#include "virusLib/microcontroller.h"
#include "virusLib/romfile.h"
#include "virusLib/romloader.h"

namespace
{
	void putError(char* _err, const size_t _len, const std::string& _msg)
	{
		if(!_err || !_len)
			return;
		const auto n = std::min(_len - 1, _msg.size());
		std::memcpy(_err, _msg.data(), n);
		_err[n] = 0;
	}

	int putString(char* _out, const size_t _len, const std::string& _s)
	{
		putError(_out, _len, _s);
		return static_cast<int>(_s.size());
	}

	// Whether the bytes are Access's firmware container as the unpacker expects it: "FORM", then
	// chunks that each fit in what is left, the first the file table ("TABL"). The unpacker itself
	// trusts all of that, and reads past its data where it is not so.
	bool isPlausibleContainer(const std::vector<uint8_t>& _d)
	{
		if(_d.size() < 16 || std::memcmp(_d.data(), "FORM", 4) != 0)
			return false;
		size_t at = 8;
		size_t chunks = 0;
		while(at + 8 <= _d.size())
		{
			const uint32_t size = (uint32_t(_d[at + 4]) << 24) | (uint32_t(_d[at + 5]) << 16) | (uint32_t(_d[at + 6]) << 8) | uint32_t(_d[at + 7]);
			if(chunks == 0 && (std::memcmp(&_d[at], "TABL", 4) != 0 || size == 0))
				return false;
			if(size > _d.size() - at - 8)
				return false;
			at += 8 + size;
			++chunks;
		}
		return chunks >= 2;
	}
}

struct KvRom
{
	virusLib::ROMFile rom;
};

struct KvSynth
{
	explicit KvSynth(const virusLib::ROMFile& _rom) : rom(_rom) {}

	virusLib::ROMFile rom;
	std::unique_ptr<virusLib::Device> device;
	std::unique_ptr<synthLib::Plugin> plugin;
};

extern "C"
{
	// A firmware file: a Virus TI's (the TI2 of the three models in it), or an A, B or C operating system.
	KvRom* kv_rom_open(const char* _path, char* _err, const size_t _errLen)
	{
		try
		{
			const std::string path(_path ? _path : "");
			// A Virus TI's firmware is Access's "FORM" container, whatever the file is called
			// (in Access's installer it is "firmware_bin", for one).
			std::vector<uint8_t> data;
			if(baseLib::filesystem::readFile(data, path) && data.size() >= 4 && std::memcmp(data.data(), "FORM", 4) == 0)
			{
				if(!isPlausibleContainer(data))
				{
					putError(_err, _errLen, "it starts as a Virus TI firmware container, but it is not a whole one");
					return nullptr;
				}
				virusLib::ROMFile ti(data, path, virusLib::DeviceModel::TI2);
				if(ti.isValid())
					return new KvRom{std::move(ti)};
				putError(_err, _errLen, "a Virus TI firmware container, but the firmware in it could not be unpacked");
				return nullptr;
			}
			auto rom = virusLib::ROMLoader::findROM(path, virusLib::DeviceModel::TI2);
			if(!rom.isValid())
				rom = virusLib::ROMLoader::findROM(path, virusLib::DeviceModel::ABC);
			if(!rom.isValid())
			{
				putError(_err, _errLen, "not a Virus firmware: a Virus TI firmware (.bin), or a Virus A, B or C operating system");
				return nullptr;
			}
			return new KvRom{std::move(rom)};
		}
		catch(const std::exception& _e)
		{
			putError(_err, _errLen, _e.what());
		}
		catch(...)
		{
			putError(_err, _errLen, "the firmware could not be read");
		}
		return nullptr;
	}

	void kv_rom_close(KvRom* _rom)
	{
		delete _rom;
	}

	// The model (virusLib::DeviceModel: 0-2 A to C, 3 Snow, 4 TI, 5 TI2) and its name.
	int kv_rom_model(const KvRom* _rom)
	{
		return static_cast<int>(_rom->rom.getModel());
	}

	int kv_rom_model_name(const KvRom* _rom, char* _out, const size_t _len)
	{
		try { return putString(_out, _len, _rom->rom.getModelName()); }
		catch(...) { return -1; }
	}

	uint32_t kv_rom_banks(const KvRom* _rom)
	{
		return _rom->rom.getNumSingleBanks();
	}

	uint32_t kv_rom_presets_per_bank(const KvRom* _rom)
	{
		return _rom->rom.getPresetsPerBank();
	}

	// A factory preset's name; -1 when there is none.
	int kv_rom_preset_name(const KvRom* _rom, const uint32_t _bank, const uint32_t _program, char* _out, const size_t _len)
	{
		try
		{
			virusLib::ROMFile::TPreset preset{};
			if(!_rom->rom.getSingle(static_cast<int>(_bank), static_cast<int>(_program), preset))
				return -1;
			return putString(_out, _len, virusLib::ROMFile::getSingleName(preset));
		}
		catch(...)
		{
			return -1;
		}
	}

	// A factory preset's data (a single: its first page holds the parameters the controllers reach); -1 when none.
	int kv_rom_preset_data(const KvRom* _rom, const uint32_t _bank, const uint32_t _program, uint8_t* _out, const size_t _len)
	{
		try
		{
			virusLib::ROMFile::TPreset preset{};
			if(!_rom->rom.getSingle(static_cast<int>(_bank), static_cast<int>(_program), preset))
				return -1;
			const auto n = std::min(_len, preset.size());
			std::memcpy(_out, preset.data(), n);
			return static_cast<int>(n);
		}
		catch(...)
		{
			return -1;
		}
	}

	// A device booted from the firmware (this takes a while), sounding at _samplerate.
	KvSynth* kv_create(const KvRom* _rom, const float _samplerate, char* _err, const size_t _errLen)
	{
		try
		{
			auto s = std::make_unique<KvSynth>(_rom->rom);

			synthLib::DeviceCreateParams params;
			params.romData = _rom->rom.getRomFileData();
			params.romName = _rom->rom.getFilename();
			params.customData = static_cast<uint32_t>(_rom->rom.getModel());
			params.hostSamplerate = _samplerate;

			s->device = std::make_unique<virusLib::Device>(params, false);
			if(!s->device->isValid())
			{
				putError(_err, _errLen, "the firmware did not boot");
				return nullptr;
			}
			s->plugin = std::make_unique<synthLib::Plugin>(s->device.get(), [](synthLib::Device*) {});
			s->plugin->setMidiClockEnabled(false);
			s->plugin->setHostSamplerate(_samplerate, 0.0f);
			return s.release();
		}
		catch(const std::exception& _e)
		{
			putError(_err, _errLen, _e.what());
		}
		catch(...)
		{
			putError(_err, _errLen, "the device could not be started");
		}
		return nullptr;
	}

	void kv_destroy(KvSynth* _s)
	{
		if(!_s)
			return;
		try
		{
			_s->plugin.reset();
			_s->device.reset();
		}
		catch(...)
		{
		}
		delete _s;
	}

	// A MIDI message (status and data bytes), _offset frames into the next block.
	void kv_midi(KvSynth* _s, const uint8_t _a, const uint8_t _b, const uint8_t _c, const uint32_t _offset)
	{
		try
		{
			_s->plugin->addMidiEvent(synthLib::SMidiEvent(synthLib::MidiEventSource::Host, _a, _b, _c, _offset));
		}
		catch(...)
		{
		}
	}

	void kv_sysex(KvSynth* _s, const uint8_t* _data, const size_t _len)
	{
		try
		{
			synthLib::SMidiEvent ev(synthLib::MidiEventSource::Host);
			ev.sysex.assign(_data, _data + _len);
			_s->plugin->addMidiEvent(ev);
		}
		catch(...)
		{
		}
	}

	// A factory preset into the edit buffer (the single that plays).
	int kv_load_preset(KvSynth* _s, const uint32_t _bank, const uint32_t _program)
	{
		try
		{
			virusLib::ROMFile::TPreset preset{};
			if(!_s->rom.getSingle(static_cast<int>(_bank), static_cast<int>(_program), preset))
				return 0;
			synthLib::SMidiEvent ev(synthLib::MidiEventSource::Host);
			ev.sysex = virusLib::Microcontroller::createSingleDump(_s->rom, virusLib::BankNumber::EditBuffer, virusLib::SINGLE, preset);
			_s->plugin->addMidiEvent(ev);
			return 1;
		}
		catch(...)
		{
			return 0;
		}
	}

	// _frames of stereo out (the main output) into _l and _r.
	void kv_process(KvSynth* _s, float* _l, float* _r, const uint32_t _frames, const float _bpm)
	{
		try
		{
			synthLib::TAudioInputs in{};
			synthLib::TAudioOutputs out{};
			out[0] = _l;
			out[1] = _r;
			_s->plugin->process(in, out, _frames, _bpm, 0.0, false, false);
		}
		catch(...)
		{
			std::fill(_l, _l + _frames, 0.0f);
			std::fill(_r, _r + _frames, 0.0f);
		}
	}

	uint32_t kv_latency(const KvSynth* _s)
	{
		return _s->plugin->getLatencyMidiToOutput();
	}

	// Its state: the bytes, or (when _out is too small) how many there are.
	size_t kv_get_state(KvSynth* _s, uint8_t* _out, const size_t _cap)
	{
		try
		{
			std::vector<uint8_t> state;
			if(!_s->plugin->getState(state, synthLib::StateTypeGlobal))
				return 0;
			if(_out && _cap >= state.size())
				std::memcpy(_out, state.data(), state.size());
			return state.size();
		}
		catch(...)
		{
			return 0;
		}
	}

	int kv_set_state(KvSynth* _s, const uint8_t* _data, const size_t _len)
	{
		try
		{
			const std::vector<uint8_t> state(_data, _data + _len);
			return _s->plugin->setState(state) ? 1 : 0;
		}
		catch(...)
		{
			return 0;
		}
	}
}
