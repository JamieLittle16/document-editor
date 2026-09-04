#include "writer_semantics_24_2.hxx"

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

#include <cstddef>
#include <memory>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace css = com::sun::star;

// Exact LibreOffice 24.2 processfactory.hxx signature.
//
// This is an internal LibreOffice ABI dependency used only by the pinned R0A
// native qualification. Keep it in this version-labelled translation unit so
// product-facing code and the native-neutral adapter surface cannot acquire the
// dependency by accident. A production implementation requires a versioned
// compatibility layer and ADR.
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

void setUnoError(std::string& error, const char* operation, const css::uno::Exception& exception)
{
    error = std::string(operation) + ": " + utf8(exception.Message);
}
} // namespace

namespace r0a
{
struct WriterSemanticView::Impl
{
    css::uno::Reference<css::text::XTextDocument> document;
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
    try
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

        auto impl = std::make_unique<Impl>();
        impl->document = found;
        return std::unique_ptr<WriterSemanticView>(new WriterSemanticView(std::move(impl)));
    }
    catch (const css::uno::Exception& exception)
    {
        setUnoError(error, "acquire Writer semantic view", exception);
        return nullptr;
    }
    catch (const std::exception& exception)
    {
        error = std::string("acquire Writer semantic view: ") + exception.what();
        return nullptr;
    }
}

ParagraphSnapshot WriterSemanticView::paragraphs(
    std::size_t maxParagraphs,
    std::size_t maxEncodedParagraphBytes) const
{
    ParagraphSnapshot snapshot;
    std::size_t encodedParagraphBytes = 0;
    snapshot.paragraphs.reserve(maxParagraphs);

    try
    {
        css::uno::Reference<css::container::XEnumerationAccess> access(
            impl_->document->getText(), css::uno::UNO_QUERY_THROW);
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

            std::string paragraph = utf8(range->getString());
            constexpr std::size_t kEncodedLengthBytes = 2;
            if (snapshot.paragraphs.size() >= maxParagraphs
                || paragraph.size() > 0xffffU
                || maxEncodedParagraphBytes < kEncodedLengthBytes
                || encodedParagraphBytes > maxEncodedParagraphBytes - kEncodedLengthBytes
                || paragraph.size()
                       > maxEncodedParagraphBytes - kEncodedLengthBytes - encodedParagraphBytes)
            {
                snapshot.status = SemanticReadStatus::LimitExceeded;
                snapshot.paragraphs.clear();
                snapshot.error = "Writer paragraph snapshot exceeds R0A semantic accumulation bound";
                return snapshot;
            }

            encodedParagraphBytes += kEncodedLengthBytes + paragraph.size();
            snapshot.paragraphs.push_back(std::move(paragraph));
        }
    }
    catch (const css::uno::Exception& exception)
    {
        snapshot.status = SemanticReadStatus::Error;
        snapshot.paragraphs.clear();
        setUnoError(snapshot.error, "enumerate Writer paragraphs", exception);
    }
    catch (const std::exception& exception)
    {
        snapshot.status = SemanticReadStatus::Error;
        snapshot.paragraphs.clear();
        snapshot.error = std::string("enumerate Writer paragraphs: ") + exception.what();
    }
    return snapshot;
}
} // namespace r0a