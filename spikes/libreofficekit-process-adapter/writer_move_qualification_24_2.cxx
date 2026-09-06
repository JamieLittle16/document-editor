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
#include <com/sun/star/lang/XEventListener.hpp>
#include <com/sun/star/text/XParagraphCursor.hpp>
#include <com/sun/star/text/XText.hpp>
#include <com/sun/star/text/XTextDocument.hpp>
#include <com/sun/star/text/XTextViewCursor.hpp>
#include <com/sun/star/text/XTextViewCursorSupplier.hpp>
#include <com/sun/star/uno/Any.hxx>
#include <com/sun/star/uno/Exception.hpp>
#include <com/sun/star/uno/Reference.hxx>
#include <com/sun/star/uno/Sequence.hxx>
#include <com/sun/star/uno/XComponentContext.hpp>
#include <com/sun/star/uno/XInterface.hpp>
#include <com/sun/star/util/URL.hpp>
#include <com/sun/star/util/URLTransformer.hpp>
#include <com/sun/star/util/XURLTransformer.hpp>
#include <cppu/unotype.hxx>
#include <rtl/string.hxx>
#include <rtl/textenc.h>
#include <rtl/ustring.hxx>

#include <algorithm>
#include <atomic>
#include <cstddef>
#include <cstring>
#include <stdexcept>
#include <string>

namespace css = com::sun::star;

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

struct WriterAuthority
{
    css::uno::Reference<css::text::XTextDocument> document;
    css::uno::Reference<css::frame::XModel> model;
    css::uno::Reference<css::frame::XDispatchProvider> dispatchProvider;
    css::uno::Reference<css::frame::XDispatchHelper> dispatchHelper;
    css::uno::Reference<css::uno::XComponentContext> context;
};

bool currentWriterAuthority(WriterAuthority& authority, std::string& error)
{
    authority.context = comphelper::getProcessComponentContext();
    if (!authority.context.is())
    {
        error = "LibreOffice process component context is null";
        return false;
    }

    auto desktop = css::frame::Desktop::create(authority.context);
    auto components = desktop->getComponents();
    if (!components.is())
    {
        error = "LibreOffice Desktop returned no component collection";
        return false;
    }

    auto enumeration = components->createEnumeration();
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
            authority.document = writer;
        }
    }

    if (writerCount != 1 || !authority.document.is())
    {
        error = "expected exactly one Writer XTextDocument in the LOK process; observed "
                + std::to_string(writerCount);
        return false;
    }

    authority.model.set(authority.document, css::uno::UNO_QUERY_THROW);
    auto controller = authority.model->getCurrentController();
    if (!controller.is())
    {
        error = "Writer model has no current controller";
        return false;
    }

    authority.dispatchProvider.set(controller->getFrame(), css::uno::UNO_QUERY_THROW);
    authority.dispatchHelper.set(
        css::frame::DispatchHelper::create(authority.context), css::uno::UNO_SET_THROW);
    return true;
}

std::string paragraphAtViewCursor(const WriterAuthority& authority)
{
    auto controller = authority.model->getCurrentController();
    css::uno::Reference<css::text::XTextViewCursorSupplier> supplier(
        controller, css::uno::UNO_QUERY_THROW);
    css::uno::Reference<css::text::XTextViewCursor> viewCursor(
        supplier->getViewCursor(), css::uno::UNO_QUERY_THROW);

    auto text = authority.document->getText();
    auto cursor = text->createTextCursorByRange(viewCursor->getStart());
    css::uno::Reference<css::text::XParagraphCursor> paragraph(cursor, css::uno::UNO_QUERY_THROW);
    paragraph->gotoStartOfParagraph(false);
    paragraph->gotoEndOfParagraph(true);
    return utf8(paragraph->getString());
}

void dispatchSynchronously(const WriterAuthority& authority, const char* command)
{
    const css::uno::Sequence<css::beans::PropertyValue> arguments;
    authority.dispatchHelper->executeDispatch(
        authority.dispatchProvider,
        rtl::OUString::createFromAscii(command),
        rtl::OUString(),
        0,
        arguments);
}

class DispatchStateProbe final : public css::frame::XStatusListener
{
public:
    css::uno::Any SAL_CALL queryInterface(const css::uno::Type& type) override
    {
        if (type == cppu::UnoType<css::frame::XStatusListener>::get())
            return css::uno::Any(css::uno::Reference<css::frame::XStatusListener>(this));
        if (type == cppu::UnoType<css::lang::XEventListener>::get())
            return css::uno::Any(css::uno::Reference<css::lang::XEventListener>(this));
        if (type == cppu::UnoType<css::uno::XInterface>::get())
        {
            return css::uno::Any(css::uno::Reference<css::uno::XInterface>(
                static_cast<css::frame::XStatusListener*>(this)));
        }
        return {};
    }

    void SAL_CALL acquire() noexcept override
    {
        referenceCount_.fetch_add(1, std::memory_order_relaxed);
    }

    void SAL_CALL release() noexcept override
    {
        if (referenceCount_.fetch_sub(1, std::memory_order_acq_rel) == 1)
            delete this;
    }

    void SAL_CALL statusChanged(const css::frame::FeatureStateEvent& event) override
    {
        received = true;
        enabled = event.IsEnabled;
    }

    void SAL_CALL disposing(const css::lang::EventObject&) override {}

    bool received = false;
    bool enabled = false;

private:
    std::atomic<sal_Int32> referenceCount_{0};
};

struct DispatchState
{
    bool present = false;
    bool received = false;
    bool enabled = false;
};

DispatchState queryDispatchState(const WriterAuthority& authority, const char* command)
{
    css::uno::Reference<css::util::XURLTransformer> transformer(
        css::util::URLTransformer::create(authority.context), css::uno::UNO_SET_THROW);
    css::util::URL url;
    url.Complete = rtl::OUString::createFromAscii(command);
    transformer->parseStrict(url);

    css::uno::Reference<css::frame::XDispatch> dispatch(
        authority.dispatchProvider->queryDispatch(url, rtl::OUString(), 0), css::uno::UNO_QUERY);
    if (!dispatch.is())
        return {};

    auto* rawProbe = new DispatchStateProbe();
    css::uno::Reference<css::frame::XStatusListener> probe(rawProbe);
    dispatch->addStatusListener(probe, url);
    const DispatchState state{true, rawProbe->received, rawProbe->enabled};
    dispatch->removeStatusListener(probe, url);
    return state;
}

bool requireMoveEnabled(const WriterAuthority& authority, std::string& error)
{
    const DispatchState state = queryDispatchState(authority, ".uno:MoveDown");
    if (!state.present)
    {
        error = "Writer frame exposes no .uno:MoveDown dispatch";
        return false;
    }
    if (!state.received)
    {
        error = "Writer .uno:MoveDown dispatch returned no status event";
        return false;
    }
    if (!state.enabled)
    {
        error = "Writer .uno:MoveDown remains disabled after list-context preparation";
        return false;
    }
    return true;
}

int reportException(
    char* error,
    std::size_t errorCapacity,
    const char* operation,
    const css::uno::Exception& exception)
{
    writeError(error, errorCapacity, std::string(operation) + ": " + utf8(exception.Message));
    return r0a::kWriterSemanticStatusError;
}
} // namespace

extern "C" int r0a_writer_semantics_prepare_paragraph_move_context(
    char* error,
    std::size_t errorCapacity)
{
    writeError(error, errorCapacity, "");
    try
    {
        WriterAuthority authority;
        std::string message;
        if (!currentWriterAuthority(authority, message))
        {
            writeError(error, errorCapacity, message);
            return r0a::kWriterSemanticStatusError;
        }

        // Writer intentionally disables its MoveDown slot for ordinary plain
        // paragraphs even though the underlying implementation is the generic
        // SwEditShell::MoveParagraph -> SwDoc::MoveParagraph path. Prepare an
        // enabled, real Writer list context using the same commands exercised by
        // LibreOffice's own Writer tests. Identity is measured only after this
        // function returns, so this setup formatting is outside the move relation.
        dispatchSynchronously(authority, ".uno:SelectAll");
        dispatchSynchronously(authority, ".uno:DefaultBullet");
        dispatchSynchronously(authority, ".uno:GoToStartOfDoc");

        const std::string currentParagraph = paragraphAtViewCursor(authority);
        if (currentParagraph != kExpectedFirstParagraph)
        {
            writeError(
                error,
                errorCapacity,
                "Writer UI cursor did not reach deterministic P0 after list preparation; observed paragraph: "
                    + currentParagraph);
            return r0a::kWriterSemanticStatusError;
        }

        if (!requireMoveEnabled(authority, message))
        {
            writeError(error, errorCapacity, message);
            return r0a::kWriterSemanticStatusError;
        }

        return r0a::kWriterSemanticStatusOk;
    }
    catch (const css::uno::Exception& exception)
    {
        return reportException(
            error, errorCapacity, "prepare Writer paragraph move context", exception);
    }
    catch (const std::exception& exception)
    {
        writeError(
            error,
            errorCapacity,
            std::string("prepare Writer paragraph move context: ") + exception.what());
        return r0a::kWriterSemanticStatusError;
    }
    catch (...)
    {
        writeError(error, errorCapacity, "prepare Writer paragraph move context: unknown native exception");
        return r0a::kWriterSemanticStatusError;
    }
}

extern "C" int r0a_writer_semantics_move_first_paragraph_down(
    char* error,
    std::size_t errorCapacity)
{
    writeError(error, errorCapacity, "");
    try
    {
        WriterAuthority authority;
        std::string message;
        if (!currentWriterAuthority(authority, message))
        {
            writeError(error, errorCapacity, message);
            return r0a::kWriterSemanticStatusError;
        }

        dispatchSynchronously(authority, ".uno:GoToStartOfDoc");
        if (paragraphAtViewCursor(authority) != kExpectedFirstParagraph)
        {
            writeError(error, errorCapacity, "Writer move did not start from deterministic P0");
            return r0a::kWriterSemanticStatusError;
        }
        if (!requireMoveEnabled(authority, message))
        {
            writeError(error, errorCapacity, message);
            return r0a::kWriterSemanticStatusError;
        }

        // This public Writer command reaches the same generic MoveParagraph path
        // in the enabled list context. The caller requires exact P1,P0,P2 text
        // and repeatable identity observations after return; dispatch completion
        // alone is never accepted as move evidence.
        dispatchSynchronously(authority, ".uno:MoveDown");
        return r0a::kWriterSemanticStatusOk;
    }
    catch (const css::uno::Exception& exception)
    {
        return reportException(error, errorCapacity, "move first Writer paragraph", exception);
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
