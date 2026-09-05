#pragma once

#include <cstddef>
#include <cstdint>

// Qualification-only ABI between the LibreOfficeKit process adapter and a
// version-pinned Writer semantic compatibility module.
//
// Keep this header free of LibreOffice/UNO types. The adapter owns the process
// protocol and LibreOfficeKit lifetime; the compatibility module owns all
// version-specific UNO knowledge and must be unloadable before LibreOfficeKit
// teardown.
//
// Probe tokens exposed by ABI v2 are intentionally view-local evidence. They
// only let the qualification harness ask whether two observations referred to
// the same UNO object while one WriterSemanticView remained alive. They are not
// document identities, paragraph IDs, persistence keys or product protocol.
namespace r0a
{
constexpr std::uint32_t kWriterSemanticModuleAbiVersion = 2;

constexpr int kWriterSemanticStatusOk = 0;
constexpr int kWriterSemanticStatusLimitExceeded = 1;
constexpr int kWriterSemanticStatusError = 2;

using WriterSemanticAbiVersionFn = std::uint32_t (*)();
using WriterSemanticAcquireFn = void* (*)(char* error, std::size_t errorCapacity);
using WriterSemanticReleaseFn = void (*)(void* view);
using WriterSemanticEncodeParagraphsFn = int (*)(
    void* view,
    std::size_t maxParagraphs,
    unsigned char* output,
    std::size_t outputCapacity,
    std::size_t* outputBytes,
    char* error,
    std::size_t errorCapacity);
using WriterSemanticEncodeIdentityParagraphsFn = WriterSemanticEncodeParagraphsFn;
using WriterSemanticSplitFirstParagraphFn = int (*)(
    void* view,
    std::uint16_t characterOffset,
    char* error,
    std::size_t errorCapacity);
using WriterSemanticMergeFirstTwoParagraphsFn = int (*)(
    void* view,
    char* error,
    std::size_t errorCapacity);
} // namespace r0a

extern "C" std::uint32_t r0a_writer_semantics_abi_version();
extern "C" void* r0a_writer_semantics_acquire(char* error, std::size_t errorCapacity);
extern "C" void r0a_writer_semantics_release(void* view);

// On success, output is a native-neutral bounded paragraph projection:
//
//   paragraph_count:u16-le
//   repeat paragraph_count times:
//       byte_length:u16-le
//       utf8_text[byte_length]
//
// The adapter wraps these bytes with its own status/command/projection-version
// and document-revision fields. The module therefore cannot define or mutate
// the process protocol by itself.
extern "C" int r0a_writer_semantics_encode_paragraphs(
    void* view,
    std::size_t maxParagraphs,
    unsigned char* output,
    std::size_t outputCapacity,
    std::size_t* outputBytes,
    char* error,
    std::size_t errorCapacity);

// Structural-identity qualification projection:
//
//   paragraph_count:u16-le
//   repeat paragraph_count times:
//       probe_token:u64-le
//       byte_length:u16-le
//       utf8_text[byte_length]
//
// A probe token is allocated from one live WriterSemanticView by comparing UNO
// XInterface references with UNO's same-object equality. It has no meaning
// after that view is released and must never be promoted into product identity.
extern "C" int r0a_writer_semantics_encode_identity_paragraphs(
    void* view,
    std::size_t maxParagraphs,
    unsigned char* output,
    std::size_t outputCapacity,
    std::size_t* outputBytes,
    char* error,
    std::size_t errorCapacity);

// Qualification-only structural edits. Each successful call performs exactly
// one authoritative mutation of the same live Writer document. The process
// adapter, not this module, advances the externally observed qualification
// revision after success.
extern "C" int r0a_writer_semantics_split_first_paragraph(
    void* view,
    std::uint16_t characterOffset,
    char* error,
    std::size_t errorCapacity);
extern "C" int r0a_writer_semantics_merge_first_two_paragraphs(
    void* view,
    char* error,
    std::size_t errorCapacity);
