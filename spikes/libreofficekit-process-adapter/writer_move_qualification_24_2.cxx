#include "writer_move_qualification_abi.hxx"
#include "writer_semantics_module_abi.hxx"

#include <com/sun/star/container/XEnumeration.hpp>
#include <com/sun/star/container/XEnumerationAccess.hpp>
#include <com/sun/star/frame/Desktop.hpp>
#include <com/sun/star/lang/XComponent.hpp>
#include <com/sun/star/lang/XUnoTunnel.hpp>
#include <com/sun/star/text/XTextDocument.hpp>
#include <com/sun/star/uno/Exception.hpp>
#include <com/sun/star/uno/Reference.hxx>
#include <com/sun/star/uno/Sequence.hxx>
#include <com/sun/star/uno/XComponentContext.hpp>
#include <rtl/string.hxx>
#include <rtl/textenc.h>
#include <rtl/ustring.hxx>
#include <sal/types.h>

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <stdexcept>
#include <string>

namespace css = com::sun::star;

// Exact LibreOffice 24.2 processfactory.hxx signature. As with the main
// semantic compatibility module, this version-specific dependency is confined
// to an unloadable qualification object and never crosses into product code.
namespace comphelper
{
css::uno::Reference<css::uno::XComponentContext> getProcessComponentContext();
}

// ---------------------------------------------------------------------------
// Pinned Writer 24.2 ABI surface
// ---------------------------------------------------------------------------
// These are deliberately declarations, not replicas of LibreOffice object
// layouts. The qualification module never allocates, copies, dereferences
// fields of, or exposes these types. It obtains the live SwXTextDocument via
// Writer's XUnoTunnel and calls only exported methods. -Wl,-z,defs makes the
// exact installed LibreOffice 24.2 library the authority for whether this ABI
// surface exists.
class SwDocShell;
class SwEditShell;
struct Tag_SwNodeOffset;

namespace o3tl
{
template <typename Value, typename Tag>
class strong_int
{
public:
    explicit constexpr strong_int(Value value) noexcept
        : value_(value)
    {
    }

private:
    Value value_;
};
} // namespace o3tl

using SwNodeOffset = o3tl::strong_int<sal_Int32, Tag_SwNodeOffset>;

class SwXTextDocument
{
public:
    static const css::uno::Sequence<sal_Int8>& getUnoTunnelId();
    SwDocShell* GetDocShell();
};

class SwDocShell
{
public:
    SwEditShell* GetEditShell();
};

class SwEditShell
{
public:
    bool MoveParagraph(SwNodeOffset offset);
};

static_assert(sizeof(SwNodeOffset) == sizeof(sal_Int32));
static_assert(alignof(SwNodeOffset) == alignof(sal_Int32));

namespace
{
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

css::uno::Reference<css::text::XTextDocument> currentWriterDocument(std::string& error)
{
    const auto context = comphelper::getProcessComponentContext();
    if (!context.is())
    {
        error = "LibreOffice process component context is null";
        return {};
    }

    auto desktop = css::frame::Desktop::create(context);
    auto components = desktop->getComponents();
    if (!components.is())
    {
        error = "LibreOffice Desktop returned no component collection";
        return {};
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
        return {};
    }

    return found;
}

SwXTextDocument* tunnelWriterImplementation(
    const css::uno::Reference<css::text::XTextDocument>& document,
    std::string& error)
{
    css::uno::Reference<css::lang::XUnoTunnel> tunnel(document, css::uno::UNO_QUERY);
    if (!tunnel.is())
    {
        error = "Writer XTextDocument does not expose XUnoTunnel";
        return nullptr;
    }

    const sal_Int64 raw = tunnel->getSomething(SwXTextDocument::getUnoTunnelId());
    if (raw == 0)
    {
        error = "Writer XUnoTunnel returned no SwXTextDocument implementation";
        return nullptr;
    }

    return reinterpret_cast<SwXTextDocument*>(static_cast<std::intptr_t>(raw));
}
} // namespace

extern "C" int r0a_writer_semantics_move_first_paragraph_down(
    char* error,
    std::size_t errorCapacity)
{
    writeError(error, errorCapacity, "");
    try
    {
        std::string message;
        auto document = currentWriterDocument(message);
        if (!document.is())
        {
            writeError(error, errorCapacity, message);
            return r0a::kWriterSemanticStatusError;
        }

        SwXTextDocument* implementation = tunnelWriterImplementation(document, message);
        if (implementation == nullptr)
        {
            writeError(error, errorCapacity, message);
            return r0a::kWriterSemanticStatusError;
        }

        SwDocShell* documentShell = implementation->GetDocShell();
        if (documentShell == nullptr)
        {
            writeError(error, errorCapacity, "Writer implementation has no live SwDocShell");
            return r0a::kWriterSemanticStatusError;
        }

        SwEditShell* editShell = documentShell->GetEditShell();
        if (editShell == nullptr)
        {
            writeError(error, errorCapacity, "Writer SwDocShell has no live SwEditShell");
            return r0a::kWriterSemanticStatusError;
        }

        // The probe has already established a fresh deterministic document and
        // the first paragraph as the active paragraph. This calls Writer's real
        // core operation, whose implementation delegates to SwDoc::MoveParagraph
        // and reports whether the move occurred. The caller then independently
        // requires exact P1,P0,P2 semantics before accepting the observation.
        if (!editShell->MoveParagraph(SwNodeOffset(1)))
        {
            writeError(error, errorCapacity, "Writer core MoveParagraph(+1) rejected verified P0");
            return r0a::kWriterSemanticStatusError;
        }

        return r0a::kWriterSemanticStatusOk;
    }
    catch (const css::uno::Exception& exception)
    {
        writeError(
            error,
            errorCapacity,
            "move first Writer paragraph: " + utf8(exception.Message));
        return r0a::kWriterSemanticStatusError;
    }
    catch (const std::exception& exception)
    {
        writeError(
            error,
            errorCapacity,
            std::string("move first Writer paragraph: ") + exception.what());
        return r0a::kWriterSemanticStatusError;
    }
    catch (...)
    {
        writeError(error, errorCapacity, "move first Writer paragraph: unknown native exception");
        return r0a::kWriterSemanticStatusError;
    }
}
