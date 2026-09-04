#define LOK_USE_UNSTABLE_API 1

#include <LibreOfficeKit/LibreOfficeKit.hxx>

#include <algorithm>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <iomanip>
#include <iostream>
#include <memory>
#include <string>
#include <vector>

namespace
{
constexpr int kCanvasWidth = 256;
constexpr int kCanvasHeight = 256;
constexpr unsigned char kSentinel = 0xA5;
constexpr char kEditMarker[] = "R0A_EDIT_MARKER_7F3D";

std::uint64_t fnv1a64(const std::vector<unsigned char>& bytes)
{
    std::uint64_t hash = 14695981039346656037ULL;
    for (const unsigned char byte : bytes)
    {
        hash ^= byte;
        hash *= 1099511628211ULL;
    }
    return hash;
}

int fail(const std::string& message)
{
    std::cerr << "probe_error=" << message << '\n';
    return 1;
}

std::string takeError(lok::Office& office)
{
    char* raw = office.getError();
    if (raw == nullptr)
        return "unknown LibreOfficeKit error";

    const std::string value(raw);
    office.freeError(raw);
    return value;
}

bool validateTextDocument(lok::Document& document, std::string& error)
{
    if (document.getDocumentType() != LOK_DOCTYPE_TEXT)
    {
        error = "loaded document is not a Writer/text document";
        return false;
    }

    document.initializeForRendering();

    long width = 0;
    long height = 0;
    document.getDocumentSize(&width, &height);
    if (width <= 0 || height <= 0)
    {
        error = "LibreOfficeKit returned non-positive document dimensions";
        return false;
    }

    const int tileMode = document.getTileMode();
    if (tileMode != LOK_TILEMODE_RGBA && tileMode != LOK_TILEMODE_BGRA)
    {
        error = "LibreOfficeKit returned an unknown tile pixel mode";
        return false;
    }

    std::vector<unsigned char> pixels(
        static_cast<std::size_t>(kCanvasWidth) * static_cast<std::size_t>(kCanvasHeight) * 4U,
        kSentinel);
    document.paintTile(
        pixels.data(),
        kCanvasWidth,
        kCanvasHeight,
        0,
        0,
        static_cast<int>(width),
        static_cast<int>(height));

    const bool bufferChanged = std::any_of(
        pixels.cbegin(), pixels.cend(), [](const unsigned char byte) { return byte != kSentinel; });
    if (!bufferChanged)
    {
        error = "paintTile did not modify caller-owned render memory";
        return false;
    }

    std::cout << "document_type=text\n";
    std::cout << "document_width_twips=" << width << '\n';
    std::cout << "document_height_twips=" << height << '\n';
    std::cout << "tile_mode=" << (tileMode == LOK_TILEMODE_RGBA ? "rgba" : "bgra") << '\n';
    std::cout << "render_hash_fnv1a64=0x" << std::hex << fnv1a64(pixels) << std::dec << '\n';
    return true;
}
} // namespace

int main(int argc, char* argv[])
{
    if (argc != 5)
    {
        std::cerr << "usage: lok-probe INSTALL_PATH PROFILE_URL INPUT.docx ROUNDTRIP.docx\n";
        return 2;
    }

    const char* installPath = argv[1];
    const char* profileUrl = argv[2];
    const char* inputPath = argv[3];
    const char* roundtripPath = argv[4];

    try
    {
        std::unique_ptr<lok::Office> office(lok::lok_cpp_init(installPath, profileUrl));
        if (!office)
            return fail("could not initialise LibreOfficeKit");

        if (char* version = office->getVersionInfo(); version != nullptr)
        {
            std::cout << "libreoffice_version_json=" << version << '\n';
            // LibreOffice 24.2 lacks the newer freeMemory() convenience wrapper.
            // freeError() delegates to the same ABI deallocator and works across our baseline.
            office->freeError(version);
        }
        else
        {
            return fail("LibreOfficeKit returned no version information");
        }

        std::unique_ptr<lok::Document> document(office->documentLoad(inputPath));
        if (!document)
            return fail("could not load input DOCX: " + takeError(*office));

        std::string validationError;
        if (!validateTextDocument(*document, validationError))
            return fail(validationError);

        if (!document->paste(
                "text/plain;charset=utf-8", kEditMarker, std::strlen(kEditMarker)))
        {
            return fail("could not apply text edit through LibreOfficeKit paste API: "
                        + takeError(*office));
        }
        std::cout << "text_edit=ok\n";
        std::cout << "text_edit_marker=" << kEditMarker << '\n';

        if (!document->saveAs(roundtripPath))
            return fail("could not save round-trip DOCX: " + takeError(*office));

        document.reset();

        std::unique_ptr<lok::Document> reopened(office->documentLoad(roundtripPath));
        if (!reopened)
            return fail("could not reopen round-trip DOCX: " + takeError(*office));

        if (reopened->getDocumentType() != LOK_DOCTYPE_TEXT)
            return fail("round-trip document reopened as a non-text document");

        long reopenedWidth = 0;
        long reopenedHeight = 0;
        reopened->initializeForRendering();
        reopened->getDocumentSize(&reopenedWidth, &reopenedHeight);
        if (reopenedWidth <= 0 || reopenedHeight <= 0)
            return fail("round-trip document reopened with invalid dimensions");

        std::cout << "roundtrip_reopen=ok\n";
        std::cout << "roundtrip_width_twips=" << reopenedWidth << '\n';
        std::cout << "roundtrip_height_twips=" << reopenedHeight << '\n';
        std::cout << "probe_status=ok\n";
        return 0;
    }
    catch (const std::exception& error)
    {
        return fail(std::string("LibreOfficeKit exception: ") + error.what());
    }
}
