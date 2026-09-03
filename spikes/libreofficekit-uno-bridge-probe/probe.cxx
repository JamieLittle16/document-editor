#define LOK_USE_UNSTABLE_API 1

#include <LibreOfficeKit/LibreOfficeKit.hxx>

#include <com/sun/star/container/XEnumeration.hpp>
#include <com/sun/star/container/XEnumerationAccess.hpp>
#include <com/sun/star/frame/Desktop.hpp>
#include <com/sun/star/lang/XComponent.hpp>
#include <com/sun/star/text/XTextDocument.hpp>
#include <com/sun/star/text/XTextRange.hpp>
#include <com/sun/star/uno/Reference.hxx>
#include <com/sun/star/uno/XComponentContext.hpp>
#include <rtl/string.hxx>
#include <rtl/textenc.h>
#include <rtl/ustring.hxx>

#include <array>
#include <cstring>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace css = com::sun::star;

// R0A discovery only.
//
// This is the exact LibreOffice 24.2 processfactory.hxx signature. It is an
// internal LibreOffice symbol, intentionally declared locally rather than copied
// into a project-wide wrapper. The probe may depend on this pinned native ABI
// solely to answer whether an embedded LibreOfficeKit document can be reached
// through the process UNO context. Production code MUST NOT depend on it without
// a later ADR and a versioned native compatibility layer.
namespace comphelper
{
css::uno::Reference<css::uno::XComponentContext> getProcessComponentContext();
}

namespace
{
constexpr std::array<const char*, 3> kExpectedParagraphs{
    "Document Editor LibreOfficeKit R0A probe",
    "This fixture is generated deterministically in CI.",
    "Stable semantic identity must be measured, not assumed.",
};
constexpr char kLiveMarker[] = "R0A_SAME_INSTANCE_5C91_";

std::string utf8(const rtl::OUString& value)
{
    const rtl::OString encoded = rtl::OUStringToOString(value, RTL_TEXTENCODING_UTF8);
    return std::string(encoded.getStr(), static_cast<std::size_t>(encoded.getLength()));
}

std::vector<std::string> enumerateParagraphs(
    const css::uno::Reference<css::text::XTextDocument>& textDocument)
{
    css::uno::Reference<css::container::XEnumerationAccess> access(
        textDocument->getText(), css::uno::UNO_QUERY_THROW);
    css::uno::Reference<css::container::XEnumeration> enumeration = access->createEnumeration();

    std::vector<std::string> paragraphs;
    while (enumeration->hasMoreElements())
    {
        css::uno::Any element = enumeration->nextElement();
        css::uno::Reference<css::uno::XInterface> interface;
        if (!(element >>= interface) || !interface.is())
            continue;

        css::uno::Reference<css::text::XTextRange> range(interface, css::uno::UNO_QUERY);
        if (range.is())
            paragraphs.push_back(utf8(range->getString()));
    }
    return paragraphs;
}

bool equalsExpected(const std::vector<std::string>& paragraphs)
{
    if (paragraphs.size() != kExpectedParagraphs.size())
        return false;
    for (std::size_t index = 0; index < paragraphs.size(); ++index)
    {
        if (paragraphs[index] != kExpectedParagraphs[index])
            return false;
    }
    return true;
}

css::uno::Reference<css::text::XTextDocument> findOnlyWriterDocument()
{
    const auto context = comphelper::getProcessComponentContext();
    if (!context.is())
        throw std::runtime_error("LibreOffice process component context is null");

    auto desktop = css::frame::Desktop::create(context);
    auto components = desktop->getComponents();
    if (!components.is())
        throw std::runtime_error("LibreOffice Desktop returned no component collection");

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
        throw std::runtime_error(
            "expected exactly one Writer XTextDocument in the LOK process; observed "
            + std::to_string(writerCount));
    return found;
}
} // namespace

int main(int argc, char* argv[])
{
    if (argc != 4)
    {
        std::cerr << "usage: lok-uno-bridge-probe INSTALL_PATH PROFILE_URL INPUT.docx\n";
        return 2;
    }

    try
    {
        std::unique_ptr<lok::Office> office(lok::lok_cpp_init(argv[1], argv[2]));
        if (!office)
            throw std::runtime_error("could not initialize LibreOfficeKit");

        std::unique_ptr<lok::Document> document(office->documentLoad(argv[3]));
        if (!document)
            throw std::runtime_error("LibreOfficeKit could not load fixture");
        if (document->getDocumentType() != LOK_DOCTYPE_TEXT)
            throw std::runtime_error("fixture did not load as Writer text document");
        document->initializeForRendering();

        const auto writer = findOnlyWriterDocument();
        const auto before = enumerateParagraphs(writer);
        if (!equalsExpected(before))
            throw std::runtime_error("UNO paragraph snapshot did not match LOK-loaded fixture");

        document->postUnoCommand(".uno:GoToStartOfDoc", nullptr, false);
        if (!document->paste(
                "text/plain;charset=utf-8",
                kLiveMarker,
                std::strlen(kLiveMarker)))
        {
            throw std::runtime_error("LOK live mutation failed");
        }

        const auto after = enumerateParagraphs(writer);
        if (after.size() != before.size())
            throw std::runtime_error("UNO paragraph count changed after prefix edit");
        if (after[0] != std::string(kLiveMarker) + before[0])
            throw std::runtime_error("same UNO Writer reference did not observe unsaved LOK edit");
        for (std::size_t index = 1; index < after.size(); ++index)
        {
            if (after[index] != before[index])
                throw std::runtime_error("LOK prefix edit unexpectedly changed another UNO paragraph");
        }

        std::cout << "same_instance_process_context=ok\n";
        std::cout << "same_instance_writer_documents=1\n";
        std::cout << "same_instance_paragraphs_before=" << before.size() << '\n';
        std::cout << "same_instance_paragraphs_after=" << after.size() << '\n';
        std::cout << "same_instance_unsaved_lok_edit_visible_in_uno=ok\n";
        std::cout << "same_instance_bridge_status=ok\n";
        return 0;
    }
    catch (const std::exception& error)
    {
        std::cerr << "same_instance_bridge_error=" << error.what() << '\n';
        return 1;
    }
}
