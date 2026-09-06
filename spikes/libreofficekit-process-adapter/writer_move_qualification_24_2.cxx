#include "writer_move_qualification_abi.hxx"
#include "writer_semantics_module_abi.hxx"

#include <com/sun/star/beans/PropertyValue.hpp>
#include <com/sun/star/container/XEnumeration.hpp>
#include <com/sun/star/container/XEnumerationAccess.hpp>
#include <com/sun/star/frame/Desktop.hpp>
#include <com/sun/star/frame/DispatchHelper.hpp>
#include <com/sun/star/frame/XDispatchHelper.hpp>
#include <com/sun/star/frame/XDispatchProvider.hpp>
#include <com/sun/star/frame/XModel.hpp>
#include <com/sun/star/lang/XComponent.hpp>
#include <com/sun/star/text/XParagraphCursor.hpp>
#include <com/sun/star/text/XText.hpp>
#include <com/sun/star/text/XTextDocument.hpp>
#include <com/sun/star/text/XTextViewCursor.hpp>
#include <com/sun/star/text/XTextViewCursorSupplier.hpp>
#include <com/sun/star/uno/Exception.hpp>
#include <com/sun/star/uno/Reference.hxx>
#include <com/sun/star/uno/Sequence.hxx>
#include <com/sun/star/uno/XComponentContext.hpp>
#include <rtl/string.hxx>
#include <rtl/textenc.h>
#include <rtl/ustring.hxx>

#include <algorithm>
#include <cstddef>
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

        // The published Text service lists XTextRangeMover as optional, but the
        // pinned Writer 24.2 SwXBodyText does not expose that interface. Writer's
        // ordinary text shell does, however, implement `.uno:MoveDown` as
        // SwWrtShell::MoveParagraph(). Drive that genuine Writer paragraph-move
        // command on the same live authority rather than falling back to a
        // delete/insert simulation.
        css::uno::Reference<css::frame::XModel> model(document, css::uno::UNO_QUERY_THROW);
        auto controller = model->getCurrentController();
        if (!controller.is())
        {
            writeError(error, errorCapacity, "Writer model has no current controller");
            return r0a::kWriterSemanticStatusError;
        }

        css::uno::Reference<css::text::XTextViewCursorSupplier> viewCursorSupplier(
            controller, css::uno::UNO_QUERY_THROW);
        css::uno::Reference<css::text::XTextViewCursor> viewCursor(
            viewCursorSupplier->getViewCursor(), css::uno::UNO_QUERY_THROW);

        auto text = document->getText();
        css::uno::Reference<css::text::XParagraphCursor> firstParagraph(
            text->createTextCursor(), css::uno::UNO_QUERY_THROW);
        firstParagraph->gotoStart(false);
        viewCursor->gotoRange(firstParagraph->getStart(), false);

        css::uno::Reference<css::frame::XDispatchProvider> dispatchProvider(
            controller->getFrame(), css::uno::UNO_QUERY_THROW);
        const auto context = comphelper::getProcessComponentContext();
        if (!context.is())
        {
            writeError(error, errorCapacity, "LibreOffice process component context disappeared");
            return r0a::kWriterSemanticStatusError;
        }

        css::uno::Reference<css::frame::XDispatchHelper> dispatchHelper(
            css::frame::DispatchHelper::create(context), css::uno::UNO_SET_THROW);
        const css::uno::Sequence<css::beans::PropertyValue> arguments;
        dispatchHelper->executeDispatch(
            dispatchProvider,
            rtl::OUString(u".uno:MoveDown"),
            rtl::OUString(),
            0,
            arguments);

        // The caller deliberately verifies exact P1,P0,P2 semantics after this
        // returns. A dispatch call by itself is never accepted as move evidence.
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
