#include "writer_semantics_24_2.hxx"
#include "writer_semantics_module_abi.hxx"

#include <dlfcn.h>

#include <array>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <limits>
#include <memory>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace
{
constexpr std::size_t kErrorBytes = 512;
constexpr std::size_t kCountBytes = 2;
constexpr std::size_t kTokenBytes = 8;
constexpr std::size_t kLengthBytes = 2;
constexpr const char* kModulePathEnvironment = "R0A_WRITER_SEMANTICS_MODULE";

template <typename Function>
Function loadFunction(void* library, const char* name)
{
    dlerror();
    void* symbol = dlsym(library, name);
    const char* error = dlerror();
    if (symbol == nullptr || error != nullptr)
    {
        throw std::runtime_error(
            std::string("semantic compatibility module is missing symbol ") + name + ": "
            + (error == nullptr ? "unknown dynamic-loader error" : error));
    }

    Function function = nullptr;
    static_assert(sizeof(function) == sizeof(symbol));
    std::memcpy(&function, &symbol, sizeof(function));
    return function;
}

std::uint16_t readU16(const unsigned char* bytes) noexcept
{
    return static_cast<std::uint16_t>(bytes[0])
           | (static_cast<std::uint16_t>(bytes[1]) << 8U);
}

std::uint64_t readU64(const unsigned char* bytes) noexcept
{
    std::uint64_t value = 0;
    for (unsigned int index = 0; index < 8; ++index)
        value |= static_cast<std::uint64_t>(bytes[index]) << (8U * index);
    return value;
}

std::string errorText(const std::array<char, kErrorBytes>& error, const char* fallback)
{
    if (error[0] == '\0')
        return fallback;
    return std::string(error.data());
}
} // namespace

namespace r0a
{
struct WriterSemanticView::Impl
{
    void* library = nullptr;
    void* view = nullptr;
    WriterSemanticReleaseFn release = nullptr;
    WriterSemanticEncodeParagraphsFn encodeParagraphs = nullptr;
    WriterSemanticEncodeIdentityParagraphsFn encodeIdentityParagraphs = nullptr;
    WriterSemanticSplitFirstParagraphFn splitFirstParagraph = nullptr;
    WriterSemanticMergeFirstTwoParagraphsFn mergeFirstTwoParagraphs = nullptr;

    ~Impl()
    {
        if (view != nullptr && release != nullptr)
            release(view);
        view = nullptr;

        if (library != nullptr)
            dlclose(library);
        library = nullptr;
    }
};

WriterSemanticView::WriterSemanticView(std::unique_ptr<Impl> impl)
    : impl_(std::move(impl))
{
}

WriterSemanticView::~WriterSemanticView() = default;
WriterSemanticView::WriterSemanticView(WriterSemanticView&&) noexcept = default;
WriterSemanticView& WriterSemanticView::operator=(WriterSemanticView&&) noexcept = default;

std::unique_ptr<WriterSemanticView> WriterSemanticView::acquire(std::string& error)
{
    error.clear();
    const char* modulePath = std::getenv(kModulePathEnvironment);
    if (modulePath == nullptr || modulePath[0] == '\0')
    {
        error = std::string("missing ") + kModulePathEnvironment
                + " for R0A Writer semantic qualification";
        return nullptr;
    }

    void* library = dlopen(modulePath, RTLD_NOW | RTLD_LOCAL);
    if (library == nullptr)
    {
        const char* loaderError = dlerror();
        error = std::string("could not load Writer semantic compatibility module: ")
                + (loaderError == nullptr ? "unknown dynamic-loader error" : loaderError);
        return nullptr;
    }

    try
    {
        const auto abiVersion = loadFunction<WriterSemanticAbiVersionFn>(
            library, "r0a_writer_semantics_abi_version");
        const auto acquireView = loadFunction<WriterSemanticAcquireFn>(
            library, "r0a_writer_semantics_acquire");
        const auto releaseView = loadFunction<WriterSemanticReleaseFn>(
            library, "r0a_writer_semantics_release");
        const auto encodeParagraphs = loadFunction<WriterSemanticEncodeParagraphsFn>(
            library, "r0a_writer_semantics_encode_paragraphs");
        const auto encodeIdentityParagraphs = loadFunction<WriterSemanticEncodeIdentityParagraphsFn>(
            library, "r0a_writer_semantics_encode_identity_paragraphs");
        const auto splitFirstParagraph = loadFunction<WriterSemanticSplitFirstParagraphFn>(
            library, "r0a_writer_semantics_split_first_paragraph");
        const auto mergeFirstTwoParagraphs = loadFunction<WriterSemanticMergeFirstTwoParagraphsFn>(
            library, "r0a_writer_semantics_merge_first_two_paragraphs");

        const std::uint32_t version = abiVersion();
        if (version != kWriterSemanticModuleAbiVersion)
        {
            error = "Writer semantic compatibility module ABI mismatch: expected "
                    + std::to_string(kWriterSemanticModuleAbiVersion) + ", observed "
                    + std::to_string(version);
            dlclose(library);
            return nullptr;
        }

        std::array<char, kErrorBytes> moduleError{};
        void* view = acquireView(moduleError.data(), moduleError.size());
        if (view == nullptr)
        {
            error = errorText(moduleError, "Writer semantic compatibility module acquisition failed");
            dlclose(library);
            return nullptr;
        }

        auto impl = std::make_unique<Impl>();
        impl->library = library;
        impl->view = view;
        impl->release = releaseView;
        impl->encodeParagraphs = encodeParagraphs;
        impl->encodeIdentityParagraphs = encodeIdentityParagraphs;
        impl->splitFirstParagraph = splitFirstParagraph;
        impl->mergeFirstTwoParagraphs = mergeFirstTwoParagraphs;
        return std::unique_ptr<WriterSemanticView>(new WriterSemanticView(std::move(impl)));
    }
    catch (const std::exception& exception)
    {
        error = exception.what();
        dlclose(library);
        return nullptr;
    }
}

ParagraphSnapshot WriterSemanticView::paragraphs(
    std::size_t maxParagraphs,
    std::size_t maxEncodedParagraphBytes) const
{
    ParagraphSnapshot snapshot;
    if (maxEncodedParagraphBytes > std::numeric_limits<std::size_t>::max() - kCountBytes)
    {
        snapshot.status = SemanticReadStatus::LimitExceeded;
        snapshot.error = "Writer semantic projection bound overflow";
        return snapshot;
    }

    std::vector<unsigned char> encoded(maxEncodedParagraphBytes + kCountBytes);
    std::array<char, kErrorBytes> moduleError{};
    std::size_t encodedBytes = 0;
    const int status = impl_->encodeParagraphs(
        impl_->view,
        maxParagraphs,
        encoded.data(),
        encoded.size(),
        &encodedBytes,
        moduleError.data(),
        moduleError.size());

    if (status == kWriterSemanticStatusLimitExceeded)
    {
        snapshot.status = SemanticReadStatus::LimitExceeded;
        snapshot.error = errorText(moduleError, "Writer semantic projection exceeded bound");
        return snapshot;
    }
    if (status != kWriterSemanticStatusOk)
    {
        snapshot.status = SemanticReadStatus::Error;
        snapshot.error = errorText(moduleError, "Writer semantic compatibility module failed");
        return snapshot;
    }
    if (encodedBytes < kCountBytes || encodedBytes > encoded.size())
    {
        snapshot.status = SemanticReadStatus::Error;
        snapshot.error = "Writer semantic compatibility module returned invalid encoded length";
        return snapshot;
    }

    const std::size_t paragraphCount = readU16(encoded.data());
    if (paragraphCount > maxParagraphs)
    {
        snapshot.status = SemanticReadStatus::Error;
        snapshot.error = "Writer semantic compatibility module exceeded paragraph contract";
        return snapshot;
    }

    snapshot.paragraphs.reserve(paragraphCount);
    std::size_t offset = kCountBytes;
    for (std::size_t index = 0; index < paragraphCount; ++index)
    {
        if (offset > encodedBytes || encodedBytes - offset < kLengthBytes)
        {
            snapshot.status = SemanticReadStatus::Error;
            snapshot.paragraphs.clear();
            snapshot.error = "Writer semantic compatibility module returned truncated paragraph length";
            return snapshot;
        }

        const std::size_t textBytes = readU16(encoded.data() + offset);
        offset += kLengthBytes;
        if (offset > encodedBytes || textBytes > encodedBytes - offset)
        {
            snapshot.status = SemanticReadStatus::Error;
            snapshot.paragraphs.clear();
            snapshot.error = "Writer semantic compatibility module returned truncated paragraph text";
            return snapshot;
        }

        snapshot.paragraphs.emplace_back(
            reinterpret_cast<const char*>(encoded.data() + offset), textBytes);
        offset += textBytes;
    }

    if (offset != encodedBytes)
    {
        snapshot.status = SemanticReadStatus::Error;
        snapshot.paragraphs.clear();
        snapshot.error = "Writer semantic compatibility module returned trailing bytes";
    }
    return snapshot;
}

IdentityProbeSnapshot WriterSemanticView::identityProbeParagraphs(
    std::size_t maxParagraphs,
    std::size_t maxEncodedBytes)
{
    IdentityProbeSnapshot snapshot;
    if (maxEncodedBytes < kCountBytes)
    {
        snapshot.status = SemanticReadStatus::LimitExceeded;
        snapshot.error = "Writer identity-probe projection bound is too small";
        return snapshot;
    }

    std::vector<unsigned char> encoded(maxEncodedBytes);
    std::array<char, kErrorBytes> moduleError{};
    std::size_t encodedBytes = 0;
    const int status = impl_->encodeIdentityParagraphs(
        impl_->view,
        maxParagraphs,
        encoded.data(),
        encoded.size(),
        &encodedBytes,
        moduleError.data(),
        moduleError.size());

    if (status == kWriterSemanticStatusLimitExceeded)
    {
        snapshot.status = SemanticReadStatus::LimitExceeded;
        snapshot.error = errorText(moduleError, "Writer identity-probe projection exceeded bound");
        return snapshot;
    }
    if (status != kWriterSemanticStatusOk)
    {
        snapshot.status = SemanticReadStatus::Error;
        snapshot.error = errorText(moduleError, "Writer identity-probe compatibility module failed");
        return snapshot;
    }
    if (encodedBytes < kCountBytes || encodedBytes > encoded.size())
    {
        snapshot.status = SemanticReadStatus::Error;
        snapshot.error = "Writer identity-probe module returned invalid encoded length";
        return snapshot;
    }

    const std::size_t paragraphCount = readU16(encoded.data());
    if (paragraphCount > maxParagraphs)
    {
        snapshot.status = SemanticReadStatus::Error;
        snapshot.error = "Writer identity-probe module exceeded paragraph contract";
        return snapshot;
    }

    snapshot.paragraphs.reserve(paragraphCount);
    std::size_t offset = kCountBytes;
    for (std::size_t index = 0; index < paragraphCount; ++index)
    {
        if (offset > encodedBytes || encodedBytes - offset < kTokenBytes + kLengthBytes)
        {
            snapshot.status = SemanticReadStatus::Error;
            snapshot.paragraphs.clear();
            snapshot.error = "Writer identity-probe module returned truncated entry";
            return snapshot;
        }

        const std::uint64_t probeToken = readU64(encoded.data() + offset);
        offset += kTokenBytes;
        if (probeToken == 0)
        {
            snapshot.status = SemanticReadStatus::Error;
            snapshot.paragraphs.clear();
            snapshot.error = "Writer identity-probe module returned reserved zero token";
            return snapshot;
        }

        const std::size_t textBytes = readU16(encoded.data() + offset);
        offset += kLengthBytes;
        if (offset > encodedBytes || textBytes > encodedBytes - offset)
        {
            snapshot.status = SemanticReadStatus::Error;
            snapshot.paragraphs.clear();
            snapshot.error = "Writer identity-probe module returned truncated paragraph text";
            return snapshot;
        }

        IdentityProbeParagraph paragraph;
        paragraph.probeToken = probeToken;
        paragraph.text.assign(
            reinterpret_cast<const char*>(encoded.data() + offset), textBytes);
        snapshot.paragraphs.push_back(std::move(paragraph));
        offset += textBytes;
    }

    if (offset != encodedBytes)
    {
        snapshot.status = SemanticReadStatus::Error;
        snapshot.paragraphs.clear();
        snapshot.error = "Writer identity-probe module returned trailing bytes";
    }
    return snapshot;
}

bool WriterSemanticView::splitFirstParagraph(
    std::uint16_t characterOffset,
    std::string& error)
{
    std::array<char, kErrorBytes> moduleError{};
    const int status = impl_->splitFirstParagraph(
        impl_->view, characterOffset, moduleError.data(), moduleError.size());
    if (status == kWriterSemanticStatusOk)
    {
        error.clear();
        return true;
    }

    error = errorText(moduleError, "Writer split structural probe failed");
    return false;
}

bool WriterSemanticView::mergeFirstTwoParagraphs(std::string& error)
{
    std::array<char, kErrorBytes> moduleError{};
    const int status = impl_->mergeFirstTwoParagraphs(
        impl_->view, moduleError.data(), moduleError.size());
    if (status == kWriterSemanticStatusOk)
    {
        error.clear();
        return true;
    }

    error = errorText(moduleError, "Writer merge structural probe failed");
    return false;
}
} // namespace r0a
