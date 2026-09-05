#include "writer_semantics_module_abi.hxx"

#include <com/sun/star/beans/XPropertySet.hpp>
#include <com/sun/star/container/XEnumeration.hpp>
#include <com/sun/star/container/XEnumerationAccess.hpp>
#include <com/sun/star/frame/Desktop.hpp>
#include <com/sun/star/lang/XComponent.hpp>
#include <com/sun/star/style/ParagraphAdjust.hpp>
#include <com/sun/star/text/ControlCharacter.hpp>
#include <com/sun/star/text/XParagraphCursor.hpp>
#include <com/sun/star/text/XText.hpp>
#include <com/sun/star/text/XTextCursor.hpp>
#include <com/sun/star/text/XTextDocument.hpp>
#include <com/sun/star/text/XTextRange.hpp>
#include <com/sun/star/uno/Exception.hpp>
#include <com/sun/star/uno/Reference.hxx>
#include <com/sun/star/uno/XComponentContext.hpp>
#include <rtl/string.hxx>
#include <rtl/textenc.h>
#include <rtl/ustring.hxx>

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <limits>
#include <memory>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace css = com::sun::star;

// Exact LibreOffice 24.2 processfactory.hxx signature.
//
// This internal ABI dependency exists only inside the version-pinned dynamic
// compatibility module. The module is loaded after LibreOfficeKit has started
// and unloaded before LibreOfficeKit teardown, so the adapter executable never
// owns LibreOffice's UNO/merged-runtime lifetime.
namespace comphelper
{
css::uno::Reference<css::uno::XComponentContext> getProcessComponentContext();
}

namespace
{
struct KnownParagraphObject
{
    css::uno::Reference<css::uno::XInterface> object;
    std::uint64_t probeToken = 0;
};

struct ObservedParagraph
{
    css::uno::Reference<css::uno::XInterface> object;
    std::string text;
};

struct WriterSemanticView
{
    css::uno::Reference<css::text::XTextDocument> document;
    std::vector<KnownParagraphObject> knownParagraphObjects;
    std::uint64_t nextProbeToken = 1;
};

std::string utf8(const rtl::OUString& value)
{
    const rtl::OString encoded = rtl::OUStringToOString(value, RTL_TEXTENCODING_UTF8);
    return std::string(encoded.getStr(), static_cast<std::size_t>(encoded.getLength()));
}

void writeError(char* output, std::size_t capacity, const std::string& message) noexcept
{
    if (output == nullptr || capacity == 0)
        return;

    const std::size_t bytes = std::min(message.size(), capacity - 1);
    if (bytes != 0)
        std::memcpy(output, message.data(), bytes);
    output[bytes] = '\0';
}

void writeU16(unsigned char* output, std::uint16_t value) noexcept
{
    output[0] = static_cast<unsigned char>(value & 0xffU);
    output[1] = static_cast<unsigned char>((value >> 8U) & 0xffU);
}

void writeU64(unsigned char* output, std::uint64_t value) noexcept
{
    for (unsigned int index = 0; index < 8; ++index)
        output[index] = static_cast<unsigned char>((value >> (8U * index)) & 0xffU);
}

std::unique_ptr<WriterSemanticView> acquireView(std::string& error)
{
    const auto context = comphelper::getProcessComponentContext();
    if (!context.is())
    {
        error = "LibreOffice process component context is null";
        return nullptr;
    }

    auto desktop = css::frame::Desktop::create(context);
    auto components = desktop->getComponents();
    if (!components.is())
    {
        error = "LibreOffice Desktop returned no component collection";
        return nullptr;
    }

    auto enumeration = components->createEnumeration();
    css::uno::Reference<css::text::XTextDocument> found;
    std::size_t writerCount = 0;
    while (enumeration->hasMoreElements())
    {
        css::uno::Any element = enumeration->nextElement();
        css::uno::Reference<css::lang::XComponent> component;
        if (!(element >>= component) || !component.is())
            continue;

        css::uno::Reference<css::text::XTextDocument> writer(component, css::uno::UNO_QUERY);
        if (writer.is())
        {
            ++writerCount;
            found = writer;
        }
    }

    if (writerCount != 1 || !found.is())
    {
        error = "expected exactly one Writer XTextDocument in the LOK process; observed "
                + std::to_string(writerCount);
        return nullptr;
    }

    auto view = std::make_unique<WriterSemanticView>();
    view->document = found;
    return view;
}

std::vector<ObservedParagraph> observeParagraphs(WriterSemanticView& view)
{
    std::vector<ObservedParagraph> paragraphs;
    css::uno::Reference<css::container::XEnumerationAccess> access(
        view.document->getText(), css::uno::UNO_QUERY_THROW);
    auto enumeration = access->createEnumeration();
    while (enumeration->hasMoreElements())
    {
        css::uno::Any element = enumeration->nextElement();
        css::uno::Reference<css::uno::XInterface> interface;
        if (!(element >>= interface) || !interface.is())
            continue;

        css::uno::Reference<css::text::XTextRange> range(interface, css::uno::UNO_QUERY);
        if (!range.is())
            continue;

        paragraphs.push_back({interface, utf8(range->getString())});
    }
    return paragraphs;
}

std::uint64_t probeTokenFor(
    WriterSemanticView& view,
    const css::uno::Reference<css::uno::XInterface>& object)
{
    for (const KnownParagraphObject& known : view.knownParagraphObjects)
    {
        if (known.object == object)
            return known.probeToken;
    }

    if (view.nextProbeToken == 0)
        throw std::runtime_error("Writer paragraph probe token space exhausted");

    const std::uint64_t token = view.nextProbeToken++;
    view.knownParagraphObjects.push_back({object, token});
    return token;
}

int encodeParagraphs(
    WriterSemanticView& view,
    std::size_t maxParagraphs,
    unsigned char* output,
    std::size_t outputCapacity,
    std::size_t& outputBytes,
    std::string& error)
{
    constexpr std::size_t kLengthBytes = 2;
    outputBytes = 0;
    if (output == nullptr || outputCapacity < kLengthBytes)
    {
        error = "semantic output buffer is too small for paragraph count";
        return r0a::kWriterSemanticStatusLimitExceeded;
    }

    const auto paragraphs = observeParagraphs(view);
    if (paragraphs.size() > maxParagraphs || paragraphs.size() > 0xffffU)
    {
        error = "Writer paragraph snapshot exceeds R0A paragraph-count bound";
        return r0a::kWriterSemanticStatusLimitExceeded;
    }

    std::size_t offset = kLengthBytes;
    for (const ObservedParagraph& observed : paragraphs)
    {
        const std::string& paragraph = observed.text;
        if (paragraph.size() > 0xffffU || offset > outputCapacity - kLengthBytes
            || paragraph.size() > outputCapacity - kLengthBytes - offset)
        {
            error = "Writer paragraph snapshot exceeds R0A semantic accumulation bound";
            return r0a::kWriterSemanticStatusLimitExceeded;
        }

        writeU16(output + offset, static_cast<std::uint16_t>(paragraph.size()));
        offset += kLengthBytes;
        if (!paragraph.empty())
            std::memcpy(output + offset, paragraph.data(), paragraph.size());
        offset += paragraph.size();
    }

    writeU16(output, static_cast<std::uint16_t>(paragraphs.size()));
    outputBytes = offset;
    return r0a::kWriterSemanticStatusOk;
}

int encodeIdentityParagraphs(
    WriterSemanticView& view,
    std::size_t maxParagraphs,
    unsigned char* output,
    std::size_t outputCapacity,
    std::size_t& outputBytes,
    std::string& error)
{
    constexpr std::size_t kCountBytes = 2;
    constexpr std::size_t kTokenBytes = 8;
    constexpr std::size_t kLengthBytes = 2;
    constexpr std::size_t kEntryFixedBytes = kTokenBytes + kLengthBytes;

    outputBytes = 0;
    if (output == nullptr || outputCapacity < kCountBytes)
    {
        error = "identity-probe output buffer is too small for paragraph count";
        return r0a::kWriterSemanticStatusLimitExceeded;
    }

    const auto paragraphs = observeParagraphs(view);
    if (paragraphs.size() > maxParagraphs || paragraphs.size() > 0xffffU)
    {
        error = "Writer identity probe exceeds R0A paragraph-count bound";
        return r0a::kWriterSemanticStatusLimitExceeded;
    }

    std::size_t offset = kCountBytes;
    for (const ObservedParagraph& observed : paragraphs)
    {
        if (observed.text.size() > 0xffffU || offset > outputCapacity
            || kEntryFixedBytes > outputCapacity - offset
            || observed.text.size() > outputCapacity - offset - kEntryFixedBytes)
        {
            error = "Writer identity probe exceeds R0A semantic accumulation bound";
            return r0a::kWriterSemanticStatusLimitExceeded;
        }

        const std::uint64_t token = probeTokenFor(view, observed.object);
        writeU64(output + offset, token);
        offset += kTokenBytes;
        writeU16(output + offset, static_cast<std::uint16_t>(observed.text.size()));
        offset += kLengthBytes;
        if (!observed.text.empty())
            std::memcpy(output + offset, observed.text.data(), observed.text.size());
        offset += observed.text.size();
    }

    writeU16(output, static_cast<std::uint16_t>(paragraphs.size()));
    outputBytes = offset;
    return r0a::kWriterSemanticStatusOk;
}

int splitFirstParagraph(
    WriterSemanticView& view,
    std::uint16_t characterOffset,
    std::string& error)
{
    if (characterOffset == 0
        || characterOffset > static_cast<std::uint16_t>(std::numeric_limits<sal_Int16>::max()))
    {
        error = "split offset must be nonzero and within the first Writer paragraph";
        return r0a::kWriterSemanticStatusError;
    }

    auto text = view.document->getText();

    css::uno::Reference<css::text::XParagraphCursor> firstParagraph(
        text->createTextCursor(), css::uno::UNO_QUERY_THROW);
    firstParagraph->gotoStart(false);
    if (!firstParagraph->gotoEndOfParagraph(true))
    {
        error = "could not determine first Writer paragraph extent";
        return r0a::kWriterSemanticStatusError;
    }
    const sal_Int32 firstParagraphLength = firstParagraph->getString().getLength();
    if (static_cast<sal_Int32>(characterOffset) > firstParagraphLength)
    {
        error = "split offset must not exceed the end of the first Writer paragraph";
        return r0a::kWriterSemanticStatusError;
    }

    auto cursor = text->createTextCursor();
    cursor->gotoStart(false);
    if (!cursor->goRight(static_cast<sal_Int16>(characterOffset), false))
    {
        error = "could not position Writer cursor at split offset";
        return r0a::kWriterSemanticStatusError;
    }

    css::uno::Reference<css::text::XTextRange> splitRange(
        cursor, css::uno::UNO_QUERY_THROW);
    text->insertControlCharacter(
        splitRange,
        css::text::ControlCharacter::PARAGRAPH_BREAK,
        false);
    return r0a::kWriterSemanticStatusOk;
}

int mergeFirstTwoParagraphs(WriterSemanticView& view, std::string& error)
{
    auto text = view.document->getText();
    css::uno::Reference<css::text::XParagraphCursor> cursor(
        text->createTextCursor(), css::uno::UNO_QUERY_THROW);
    cursor->gotoStart(false);
    if (!cursor->gotoEndOfParagraph(false))
    {
        error = "could not reach end of first Writer paragraph";
        return r0a::kWriterSemanticStatusError;
    }
    if (!cursor->goRight(1, true))
    {
        error = "Writer document has no second paragraph to merge";
        return r0a::kWriterSemanticStatusError;
    }

    css::uno::Reference<css::text::XTextRange> mergeRange(
        cursor, css::uno::UNO_QUERY_THROW);
    mergeRange->setString(rtl::OUString());
    return r0a::kWriterSemanticStatusOk;
}

int centerFirstParagraph(WriterSemanticView& view, std::string& error)
{
    const auto paragraphs = observeParagraphs(view);
    if (paragraphs.empty())
    {
        error = "Writer document has no first paragraph to format";
        return r0a::kWriterSemanticStatusError;
    }

    css::uno::Reference<css::beans::XPropertySet> properties(
        paragraphs.front().object, css::uno::UNO_QUERY_THROW);
    const rtl::OUString propertyName = rtl::OUString::createFromAscii("ParaAdjust");
    const sal_Int16 centered = static_cast<sal_Int16>(css::style::ParagraphAdjust_CENTER);

    css::uno::Any value;
    value <<= centered;
    properties->setPropertyValue(propertyName, value);

    sal_Int16 observed = -1;
    const css::uno::Any readBack = properties->getPropertyValue(propertyName);
    if (!(readBack >>= observed) || observed != centered)
    {
        error = "Writer ParaAdjust CENTER formatting did not read back after mutation";
        return r0a::kWriterSemanticStatusError;
    }

    return r0a::kWriterSemanticStatusOk;
}

template <typename Operation>
int runModuleOperation(
    const char* operationName,
    char* error,
    std::size_t errorCapacity,
    Operation&& operation)
{
    writeError(error, errorCapacity, "");
    try
    {
        std::string message;
        const int status = operation(message);
        if (status != r0a::kWriterSemanticStatusOk)
            writeError(error, errorCapacity, message);
        return status;
    }
    catch (const css::uno::Exception& exception)
    {
        writeError(error, errorCapacity, std::string(operationName) + ": " + utf8(exception.Message));
        return r0a::kWriterSemanticStatusError;
    }
    catch (const std::exception& exception)
    {
        writeError(error, errorCapacity, std::string(operationName) + ": " + exception.what());
        return r0a::kWriterSemanticStatusError;
    }
    catch (...)
    {
        writeError(error, errorCapacity, std::string(operationName) + ": unknown native exception");
        return r0a::kWriterSemanticStatusError;
    }
}
} // namespace

extern "C" std::uint32_t r0a_writer_semantics_abi_version()
{
    return r0a::kWriterSemanticModuleAbiVersion;
}

extern "C" void* r0a_writer_semantics_acquire(char* error, std::size_t errorCapacity)
{
    writeError(error, errorCapacity, "");
    try
    {
        std::string message;
        auto view = acquireView(message);
        if (!view)
        {
            writeError(error, errorCapacity, message);
            return nullptr;
        }
        return view.release();
    }
    catch (const css::uno::Exception& exception)
    {
        writeError(error, errorCapacity, "acquire Writer semantic view: " + utf8(exception.Message));
        return nullptr;
    }
    catch (const std::exception& exception)
    {
        writeError(error, errorCapacity, std::string("acquire Writer semantic view: ") + exception.what());
        return nullptr;
    }
    catch (...)
    {
        writeError(error, errorCapacity, "acquire Writer semantic view: unknown native exception");
        return nullptr;
    }
}

extern "C" void r0a_writer_semantics_release(void* view)
{
    delete static_cast<WriterSemanticView*>(view);
}

extern "C" int r0a_writer_semantics_encode_paragraphs(
    void* view,
    std::size_t maxParagraphs,
    unsigned char* output,
    std::size_t outputCapacity,
    std::size_t* outputBytes,
    char* error,
    std::size_t errorCapacity)
{
    if (outputBytes != nullptr)
        *outputBytes = 0;
    if (view == nullptr || outputBytes == nullptr)
    {
        writeError(error, errorCapacity, "invalid Writer semantic module arguments");
        return r0a::kWriterSemanticStatusError;
    }

    return runModuleOperation(
        "enumerate Writer paragraphs",
        error,
        errorCapacity,
        [&](std::string& message) {
            std::size_t encodedBytes = 0;
            const int status = encodeParagraphs(
                *static_cast<WriterSemanticView*>(view),
                maxParagraphs,
                output,
                outputCapacity,
                encodedBytes,
                message);
            if (status == r0a::kWriterSemanticStatusOk)
                *outputBytes = encodedBytes;
            return status;
        });
}

extern "C" int r0a_writer_semantics_encode_identity_paragraphs(
    void* view,
    std::size_t maxParagraphs,
    unsigned char* output,
    std::size_t outputCapacity,
    std::size_t* outputBytes,
    char* error,
    std::size_t errorCapacity)
{
    if (outputBytes != nullptr)
        *outputBytes = 0;
    if (view == nullptr || outputBytes == nullptr)
    {
        writeError(error, errorCapacity, "invalid Writer identity-probe module arguments");
        return r0a::kWriterSemanticStatusError;
    }

    return runModuleOperation(
        "enumerate Writer identity-probe paragraphs",
        error,
        errorCapacity,
        [&](std::string& message) {
            std::size_t encodedBytes = 0;
            const int status = encodeIdentityParagraphs(
                *static_cast<WriterSemanticView*>(view),
                maxParagraphs,
                output,
                outputCapacity,
                encodedBytes,
                message);
            if (status == r0a::kWriterSemanticStatusOk)
                *outputBytes = encodedBytes;
            return status;
        });
}

extern "C" int r0a_writer_semantics_split_first_paragraph(
    void* view,
    std::uint16_t characterOffset,
    char* error,
    std::size_t errorCapacity)
{
    if (view == nullptr)
    {
        writeError(error, errorCapacity, "invalid Writer split-probe module arguments");
        return r0a::kWriterSemanticStatusError;
    }

    return runModuleOperation(
        "split first Writer paragraph",
        error,
        errorCapacity,
        [&](std::string& message) {
            return splitFirstParagraph(
                *static_cast<WriterSemanticView*>(view), characterOffset, message);
        });
}

extern "C" int r0a_writer_semantics_merge_first_two_paragraphs(
    void* view,
    char* error,
    std::size_t errorCapacity)
{
    if (view == nullptr)
    {
        writeError(error, errorCapacity, "invalid Writer merge-probe module arguments");
        return r0a::kWriterSemanticStatusError;
    }

    return runModuleOperation(
        "merge first two Writer paragraphs",
        error,
        errorCapacity,
        [&](std::string& message) {
            return mergeFirstTwoParagraphs(*static_cast<WriterSemanticView*>(view), message);
        });
}

extern "C" int r0a_writer_semantics_center_first_paragraph(
    void* view,
    char* error,
    std::size_t errorCapacity)
{
    if (view == nullptr)
    {
        writeError(error, errorCapacity, "invalid Writer paragraph-format module arguments");
        return r0a::kWriterSemanticStatusError;
    }

    return runModuleOperation(
        "center first Writer paragraph",
        error,
        errorCapacity,
        [&](std::string& message) {
            return centerFirstParagraph(*static_cast<WriterSemanticView*>(view), message);
        });
}
