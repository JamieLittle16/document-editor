#pragma once

#include <cstddef>
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

private:
    struct Impl;

    explicit WriterSemanticView(std::unique_ptr<Impl> impl);

    std::unique_ptr<Impl> impl_;
};
} // namespace r0a
