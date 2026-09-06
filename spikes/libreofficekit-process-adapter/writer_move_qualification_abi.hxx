#pragma once

#include <cstddef>

// Qualification-only extension to the pinned Writer semantic module.
//
// This deliberately remains separate from writer_semantics_module_abi.hxx:
// paragraph movement is currently an empirical identity experiment, not a
// product engine operation and not part of the stable semantic-module ABI.
// No native pointer, UNO reference or probe token crosses this boundary.
namespace r0a
{
using WriterPrepareParagraphMoveContextFn = int (*)(char* error, std::size_t errorCapacity);
using WriterMoveFirstParagraphDownFn = int (*)(char* error, std::size_t errorCapacity);
} // namespace r0a

// Prepare a Writer context in which its public MoveDown command deliberately
// exercises the real paragraph-node move path. The identity baseline is taken
// only after this preparation succeeds, so setup formatting is excluded from
// the measured move relation.
extern "C" int r0a_writer_semantics_prepare_paragraph_move_context(
    char* error,
    std::size_t errorCapacity);

extern "C" int r0a_writer_semantics_move_first_paragraph_down(
    char* error,
    std::size_t errorCapacity);
