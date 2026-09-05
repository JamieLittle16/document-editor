#include "writer_semantics_module_abi.hxx"

#include <com/sun/star/container/XEnumeration.hpp>
#include <com/sun/star/container/XEnumerationAccess.hpp>
#include <com/sun/star/frame/Desktop.hpp>
#include <com/sun/star/lang/XComponent.hpp>
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
#include <memory>
#include <new>
#include <string>

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
struct WriterSemanticView
{
    css::uno::Reference<css::text::XTextDocument> document;
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

    std::size_t offset = kLengthBytes;
    std::size_t paragraphCount = 0;
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

        const std::string paragraph = utf8(range->getString());
        if (paragraphCount >= maxParagraphs || paragraphCount >= 0xffffU
            || paragraph.size() > 0xffffU || offset > outputCapacity - kLengthBytes
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
        ++paragraphCount;
    }

    writeU16(output, static_cast<std::uint16_t>(paragraphCount));
    outputBytes = offset;
    return r0a::kWriterSemanticStatusOk;
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
        writeError(
            error,
            errorCapacity,
            "acquire Writer semantic view: " + utf8(exception.Message));
        return nullptr;
    }
    catch (const std::exception& exception)
    {
        writeError(
            error,
            errorCapacity,
            std::string("acquire Writer semantic view: ") + exception.what());
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
    writeError(error, errorCapacity, "");
    if (view == nullptr || outputBytes == nullptr)
    {
        writeError(error, errorCapacity, "invalid Writer semantic module arguments");
        return r0a::kWriterSemanticStatusError;
    }

    try
    {
        std::string message;
        std::size_t encodedBytes = 0;
        const int status = encodeParagraphs(
            *static_cast<WriterSemanticView*>(view),
            maxParagraphs,
            output,
            outputCapacity,
            encodedBytes,
            message);
        if (status != r0a::kWriterSemanticStatusOk)
        {
            writeError(error, errorCapacity, message);
            return status;
        }

        *outputBytes = encodedBytes;
        return r0a::kWriterSemanticStatusOk;
    }
    catch (const css::uno::Exception& exception)
    {
        writeError(
            error,
            errorCapacity,
            "enumerate Writer paragraphs: " + utf8(exception.Message));
        return r0a::kWriterSemanticStatusError;
    }
    catch (const std::exception& exception)
    {
        writeError(
            error,
            errorCapacity,
            std::string("enumerate Writer paragraphs: ") + exception.what());
        return r0a::kWriterSemanticStatusError;
    }
    catch (...)
    {
        writeError(error, errorCapacity, "enumerate Writer paragraphs: unknown native exception");
        return r0a::kWriterSemanticStatusError;
    }
}
