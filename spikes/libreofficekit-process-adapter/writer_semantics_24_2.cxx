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

#include <dlfcn.h>

#include <cstddef>
#include <cstring>
#include <memory>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace css = com::sun::star;

namespace
{
// Exact LibreOffice 24.2 merged-library ABI dependency.
//
// Ubuntu's no-GUI LibreOffice package merges comphelper into libmergedlo.so, and
// LibreOfficeKit owns that library's dynamic lifetime. Do not link libmergedlo
// into the adapter executable: doing so keeps its process-global static state
// alive beyond LOK teardown and changes destruction order. Instead, look up the
// exported 24.2 comphelper entry point on the library instance LOK already owns.
// This remains qualification-only machinery behind the version-labelled TU.
using GetProcessComponentContext =
    css::uno::Reference<css::uno::XComponentContext> (*)();

class DynamicLibraryHandle
{
public:
    explicit DynamicLibraryHandle(void* value)
        : value_(value)
    {
    }

    ~DynamicLibraryHandle()
    {
        if (value_ != nullptr)
            dlclose(value_);
    }

    DynamicLibraryHandle(const DynamicLibraryHandle&) = delete;
    DynamicLibraryHandle& operator=(const DynamicLibraryHandle&) = delete;

    [[nodiscard]] void* get() const noexcept { return value_; }

private:
    void* value_;
};

css::uno::Reference<css::uno::XComponentContext> processComponentContext()
{
    dlerror();
    DynamicLibraryHandle merged(
        dlopen("libmergedlo.so", RTLD_LAZY | RTLD_LOCAL | RTLD_NOLOAD));
    if (merged.get() == nullptr)
    {
        const char* error = dlerror();
        throw std::runtime_error(
            std::string("LibreOfficeKit merged runtime is not loaded: ")
            + (error == nullptr ? "unknown dynamic-loader error" : error));
    }

    dlerror();
    void* symbol = dlsym(
        merged.get(),
        "_ZN10comphelper26getProcessComponentContextEv");
    if (symbol == nullptr)
    {
        const char* error = dlerror();
        throw std::runtime_error(
            std::string("LibreOffice 24.2 process-context symbol is unavailable: ")
            + (error == nullptr ? "unknown dynamic-loader error" : error));
    }

    GetProcessComponentContext getContext = nullptr;
    static_assert(sizeof(getContext) == sizeof(symbol));
    std::memcpy(&getContext, &symbol, sizeof(getContext));
    return getContext();
}

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
        const auto context = processComponentContext();
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
