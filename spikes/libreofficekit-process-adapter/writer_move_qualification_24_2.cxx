#include "writer_move_qualification_abi.hxx"
#include "writer_semantics_module_abi.hxx"

#include <com/sun/star/beans/PropertyValue.hpp>
#include <com/sun/star/container/XEnumeration.hpp>
#include <com/sun/star/container/XEnumerationAccess.hpp>
#include <com/sun/star/frame/Desktop.hpp>
#include <com/sun/star/frame/DispatchHelper.hpp>
#include <com/sun/star/frame/FeatureStateEvent.hpp>
#include <com/sun/star/frame/XDispatch.hpp>
#include <com/sun/star/frame/XDispatchHelper.hpp>
#include <com/sun/star/frame/XDispatchProvider.hpp>
#include <com/sun/star/frame/XModel.hpp>
#include <com/sun/star/frame/XStatusListener.hpp>
#include <com/sun/star/lang/EventObject.hpp>
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
#include <com/sun/star/util/URL.hpp>
#include <com/sun/star/util/URLTransformer.hpp>
#include <com/sun/star/util/XURLTransformer.hpp>
#include <cppuhelper/implbase.hxx>
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
constexpr const char* kExpectedFirstParagraph = "Document Editor LibreOfficeKit R0A probe";

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

std::string paragraphAtViewCursor(
    const css::uno::Reference<css::text::XTextDocument>& document,
    const css::uno::Reference<css::frame::XModel>& model)
{
    auto controller = model->getCurrentController();
    css::uno::Reference<css::text::XTextViewCursorSupplier> supplier(
        controller, css::uno::UNO_QUERY_THROW);
    css::uno::Reference<css::text::XTextViewCursor> viewCursor(
        supplier->getViewCursor(), css::uno::UNO_QUERY_THROW);

    auto text = document->getText();
    auto cursor = text->createTextCursorByRange(viewCursor->getStart());
    css::uno::Reference<css::text::XParagraphCursor> paragraph(cursor, css::uno::UNO_QUERY_THROW);
    paragraph->gotoStartOfParagraph(false);
    paragraph->gotoEndOfParagraph(true);
    return utf8(paragraph->getString());
}

void dispatchSynchronously(
    const css::uno::Reference<css::frame::XDispatchHelper>& helper,
    const css::uno::Reference<css::frame::XDispatchProvider>& provider,
    const char* command)
{
    const css::uno::Sequence<css::beans::PropertyValue> arguments;
    helper->executeDispatch(
        provider,
        rtl::OUString::createFromAscii(command),
        rtl::OUString(),
        0,
        arguments);
}

class DispatchStateProbe final : public cppu::WeakImplHelper<css::frame::XStatusListener>
{
public:
    void SAL_CALL statusChanged(const css::frame::FeatureStateEvent& event) override
    {
        received = true;
        enabled = event.IsEnabled;
    }

    void SAL_CALL disposing(const css::lang::EventObject&) override {}

    bool received = false;
    bool enabled = false;
};

struct DispatchState
{
    bool present = false;
    bool received = false;
    bool enabled = false;
};

DispatchState queryDispatchState(
    const css::uno::Reference<css::frame::XDispatchProvider>& provider,
    const css::uno::Reference<css::uno::XComponentContext>& context,
    const char* command)
{
    css::uno::Reference<css::util::XURLTransformer> transformer(
        css::util::URLTransformer::create(context), css::uno::UNO_SET_THROW);
    css::util::URL url;
    url.Complete = rtl::OUString::createFromAscii(command);
    transformer->parseStrict(url);

    css::uno::Reference<css::frame::XDispatch> dispatch(
        provider->queryDispatch(url, rtl::OUString(), 0), css::uno::UNO_QUERY);
    if (!dispatch.is())
        return {};

    rtl::Reference<DispatchStateProbe> probe = new DispatchStateProbe();
    dispatch->addStatusListener(probe, url);
    const DispatchState state{true, probe->received, probe->enabled};
    dispatch->removeStatusListener(probe, url);
    return state;
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

        css::uno::Reference<css::frame::XModel> model(document, css::uno::UNO_QUERY_THROW);
        auto controller = model->getCurrentController();
        if (!controller.is())
        {
            writeError(error, errorCapacity, "Writer model has no current controller");
            return r0a::kWriterSemanticStatusError;
        }

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

        // Use the Writer shell itself to put the UI cursor at document start,
        // then verify the semantic paragraph under that cursor before interpreting
        // any move result.
        dispatchSynchronously(dispatchHelper, dispatchProvider, ".uno:GoToStartOfDoc");

        const std::string currentParagraph = paragraphAtViewCursor(document, model);
        if (currentParagraph != kExpectedFirstParagraph)
        {
            writeError(
                error,
                errorCapacity,
                "Writer UI cursor did not reach deterministic P0 before move; observed paragraph: "
                    + currentParagraph);
            return r0a::kWriterSemanticStatusError;
        }

        // A disabled Sfx slot can still resolve to an XDispatch and turn a
        // synchronous dispatch into a clean no-op. Observe the command's real
        // frame state before invoking it so CI distinguishes command ineligibility
        // from mutation semantics.
        const DispatchState moveState = queryDispatchState(
            dispatchProvider, context, ".uno:MoveDown");
        if (!moveState.present)
        {
            writeError(error, errorCapacity, "Writer frame exposes no .uno:MoveDown dispatch");
            return r0a::kWriterSemanticStatusError;
        }
        if (!moveState.received)
        {
            writeError(error, errorCapacity, "Writer .uno:MoveDown dispatch returned no status event");
            return r0a::kWriterSemanticStatusError;
        }
        if (!moveState.enabled)
        {
            writeError(error, errorCapacity, "Writer .uno:MoveDown dispatch is disabled at verified P0");
            return r0a::kWriterSemanticStatusError;
        }

        // DispatchHelper forces SynchronMode=true and waits for notification when
        // supported. No VCL scheduler/event-loop pumping is used here.
        dispatchSynchronously(dispatchHelper, dispatchProvider, ".uno:MoveDown");

        // The caller deliberately verifies exact P1,P0,P2 semantics after this
        // returns. Dispatch completion is never accepted as move evidence by
        // itself.
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
