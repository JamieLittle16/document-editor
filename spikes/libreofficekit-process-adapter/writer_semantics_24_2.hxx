#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>
#include <string>
#include <vector>

namespace r0a
{
enum class SemanticReadStatus
{
    Ok,
    LimitExceeded,
    Error,
};

struct ParagraphSnapshot
{
    SemanticReadStatus status = SemanticReadStatus::Ok;
    std::vector<std::string> paragraphs;
    std::string error;
};

struct IdentityProbeParagraph
{
    // View-local qualification evidence only. This is deliberately not named an
    // ID: it has no meaning after the WriterSemanticView is released.
    std::uint64_t probeToken = 0;
    std::string text;
};

struct IdentityProbeSnapshot
{
    SemanticReadStatus status = SemanticReadStatus::Ok;
    std::vector<IdentityProbeParagraph> paragraphs;
    std::string error;
};

class WriterSemanticView
{
public:
    static std::unique_ptr<WriterSemanticView> acquire(std::string& error);

    ~WriterSemanticView();

    WriterSemanticView(const WriterSemanticView&) = delete;
    WriterSemanticView& operator=(const WriterSemanticView&) = delete;
    WriterSemanticView(WriterSemanticView&&) noexcept;
    WriterSemanticView& operator=(WriterSemanticView&&) noexcept;

    ParagraphSnapshot paragraphs(
        std::size_t maxParagraphs,
        std::size_t maxEncodedParagraphBytes) const;

    IdentityProbeSnapshot identityProbeParagraphs(
        std::size_t maxParagraphs,
        std::size_t maxEncodedBytes);

    bool splitFirstParagraph(std::uint16_t characterOffset, std::string& error);
    bool mergeFirstTwoParagraphs(std::string& error);
    bool centerFirstParagraph(std::string& error);

private:
    struct Impl;

    explicit WriterSemanticView(std::unique_ptr<Impl> impl);

    std::unique_ptr<Impl> impl_;
};
} // namespace r0a
