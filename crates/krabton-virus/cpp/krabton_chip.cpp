// A bare DSP56300 for Krabton's own firmware: the emulator's core, memory and
// JIT, no Virus and no peripherals (Krabton reads and writes the chip's
// memory itself, as a host would). Its own assembler turns firmware source
// into program words, one instruction at a time; its disassembler turns them
// back, for the tests. Nothing throws across the interface.
//
// Copyright (C) Krabton contributors. GPL-3.0-only, as the code it calls.

#include <cstdint>
#include <cstring>
#include <memory>
#include <string>
#include <vector>

#include "dsp56kEmu/assembler.h"
#include "dsp56kEmu/disasm.h"
#include "dsp56kEmu/dsp.h"
#include "dsp56kEmu/memory.h"
#include "dsp56kEmu/opcodes.h"
#include "dsp56kEmu/peripherals.h"
#include "dsp56kEmu/utils.h"

namespace
{
	void putText(char* _out, const size_t _len, const std::string& _s)
	{
		if(!_out || !_len)
			return;
		const auto n = std::min(_len - 1, _s.size());
		std::memcpy(_out, _s.data(), n);
		_out[n] = 0;
	}

	const dsp56k::Assembler& assembler()
	{
		static const dsp56k::Assembler a;
		return a;
	}

	const char* errorText(const dsp56k::AssembleError _e)
	{
		switch(_e)
		{
		case dsp56k::AssembleError::OK: return "ok";
		case dsp56k::AssembleError::InvalidInstruction: return "unknown instruction";
		case dsp56k::AssembleError::InvalidRegister: return "invalid register";
		case dsp56k::AssembleError::InvalidAddressingMode: return "invalid addressing mode";
		case dsp56k::AssembleError::InvalidAddress: return "invalid address";
		case dsp56k::AssembleError::ValueOutOfRange: return "value out of range";
		case dsp56k::AssembleError::InvalidBitNumber: return "invalid bit number";
		case dsp56k::AssembleError::InvalidConditionCode: return "invalid condition code";
		case dsp56k::AssembleError::InvalidMemoryArea: return "invalid memory area";
		case dsp56k::AssembleError::InvalidImmediate: return "invalid immediate";
		case dsp56k::AssembleError::InvalidOperandCount: return "wrong number of operands";
		case dsp56k::AssembleError::InvalidSyntax: return "invalid syntax";
		case dsp56k::AssembleError::AmbiguousInstruction: return "ambiguous instruction";
		default: return "internal error";
		}
	}

	constexpr dsp56k::TWord g_bridged = 0x020000;
}

struct KdChip
{
	explicit KdChip(const dsp56k::TWord _memSize)
	{
		const size_t bytes = dsp56k::alignedSize<dsp56k::DSP>() + dsp56k::alignedSize<dsp56k::Memory>() + dsp56k::Memory::calcMemSize(_memSize, g_bridged) * sizeof(uint32_t);
		buffer.resize(dsp56k::alignedSize(bytes) + dsp56k::g_cacheLineSize);
		auto* buf = dsp56k::alignedAddress(buffer.data());
		auto* bufDsp = buf;
		auto* bufMem = bufDsp + dsp56k::alignedSize<dsp56k::DSP>();
		auto* bufSpace = bufMem + dsp56k::alignedSize<dsp56k::Memory>();
		memory = new (bufMem) dsp56k::Memory(validator, _memSize, _memSize, g_bridged, reinterpret_cast<dsp56k::TWord*>(bufSpace));
		dsp = new (bufDsp) dsp56k::DSP(*memory, &periphX, &periphY);
	}

	~KdChip()
	{
		dsp->~DSP();
		memory->~Memory();
	}

	dsp56k::DefaultMemoryValidator validator;
	dsp56k::PeripheralsNop periphX;
	dsp56k::PeripheralsNop periphY;
	std::vector<uint8_t> buffer;
	dsp56k::Memory* memory = nullptr;
	dsp56k::DSP* dsp = nullptr;
};

extern "C"
{
	// A chip with `_memSize` words of P, X and Y memory each.
	KdChip* kd_create(const uint32_t _memSize, char* _err, const size_t _errLen)
	{
		try
		{
			return new KdChip(_memSize);
		}
		catch(const std::exception& _e) { putText(_err, _errLen, _e.what()); }
		catch(...) { putText(_err, _errLen, "the chip could not be made"); }
		return nullptr;
	}

	void kd_destroy(KdChip* _c)
	{
		try { delete _c; } catch(...) {}
	}

	// One instruction to its words: how many (1 or 2), or 0 with what is wrong.
	int kd_assemble(const char* _text, uint32_t* _words, char* _err, const size_t _errLen)
	{
		try
		{
			const auto r = assembler().assemble(_text ? _text : "");
			if(!r.success())
			{
				putText(_err, _errLen, errorText(r.error));
				return 0;
			}
			_words[0] = r.word[0];
			_words[1] = r.word[1];
			return static_cast<int>(r.wordCount);
		}
		catch(const std::exception& _e) { putText(_err, _errLen, _e.what()); }
		catch(...) { putText(_err, _errLen, "the assembler failed"); }
		return 0;
	}

	// Words back to text (at `_pc`); how many words the instruction has.
	int kd_disassemble(const uint32_t _a, const uint32_t _b, const uint32_t _pc, char* _out, const size_t _len)
	{
		try
		{
			static const dsp56k::Opcodes opcodes;
			dsp56k::Disassembler d(opcodes);
			std::string s;
			const auto n = d.disassemble(s, _a, _b, 0, 0, _pc);
			putText(_out, _len, s);
			return static_cast<int>(n);
		}
		catch(...) { putText(_out, _len, "?"); }
		return 0;
	}

	// Writes words to memory: area 0 is P, 1 X, 2 Y.
	void kd_write(KdChip* _c, const int _area, const uint32_t _addr, const uint32_t* _words, const size_t _count)
	{
		if(!_c)
			return;
		try
		{
			for(size_t i = 0; i < _count; ++i)
			{
				const auto a = static_cast<dsp56k::TWord>(_addr + i);
				const auto v = _words[i] & 0xffffff;
				if(_area == 0)
					_c->dsp->memWriteP(a, v);
				else
					_c->memory->set(_area == 1 ? dsp56k::MemArea_X : dsp56k::MemArea_Y, a, v);
			}
		}
		catch(...) {}
	}

	void kd_read(KdChip* _c, const int _area, const uint32_t _addr, uint32_t* _out, const size_t _count)
	{
		if(!_c)
			return;
		try
		{
			const auto area = _area == 0 ? dsp56k::MemArea_P : _area == 1 ? dsp56k::MemArea_X : dsp56k::MemArea_Y;
			for(size_t i = 0; i < _count; ++i)
				_out[i] = _c->memory->get(area, static_cast<dsp56k::TWord>(_addr + i)) & 0xffffff;
		}
		catch(...) {}
	}

	// Runs from `_pc` until the chip is at `_until`: how many times it was
	// stepped (the JIT steps whole blocks), or -1 if it did not get there in
	// `_maxSteps`.
	int64_t kd_run(KdChip* _c, const uint32_t _pc, const uint32_t _until, const uint64_t _maxSteps)
	{
		if(!_c)
			return -1;
		try
		{
			auto& dsp = *_c->dsp;
			dsp.setPC(_pc);
			for(uint64_t i = 0; i < _maxSteps; ++i)
			{
				if(dsp.getPC().toWord() == _until)
					return static_cast<int64_t>(i);
				dsp.exec();
			}
		}
		catch(...) {}
		return -1;
	}

	// The chip's cycle count so far.
	uint64_t kd_cycles(const KdChip* _c)
	{
		return _c ? static_cast<uint64_t>(_c->dsp->getCycles()) : 0;
	}
}
