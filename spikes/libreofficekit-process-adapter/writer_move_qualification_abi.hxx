#pragma once

#include <cstddef>

// Qualification-only extension to the pinned Writer semantic module.
//
// This deliberately remains separate from writer_semantics_module_abi.hxx:
// paragraph movement is currently an empirical identity experiment, not a
// product engine operation and not part of the stable semantic-module ABI.
// The function acts on the one authoritative Writer document already owned by
// the LibreOfficeKit process and returns the same bounded status convention as
// the base semantic module.
namespace r0a
{
using WriterMoveFirstParagraphDownFn = int (*)(char* error, std::size_t errorCapacity);
} // namespace r0a

extern "C" int r0a_writer_semantics_move_first_paragraph_down(
    char* error,
    std::size_t errorCapacity);
