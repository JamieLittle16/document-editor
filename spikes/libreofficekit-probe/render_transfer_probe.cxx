#define LOK_USE_UNSTABLE_API 1

#include <LibreOfficeKit/LibreOfficeKit.hxx>

#include <algorithm>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <iostream>
#include <limits>
#include <memory>
#include <string>
#include <vector>

namespace
{
constexpr int kBytesPerPixel = 4;
constexpr int kLogicalViewportWidthPx = 1024;
constexpr int kLogicalViewportHeightPx = 768;
constexpr int kTwipsPerLogicalPixel = 15; // 96 DPI: 1440 twips / 96 pixels.
constexpr int kLogicalTilePixels = 256;
constexpr int kLogicalTileTwips = kLogicalTilePixels * kTwipsPerLogicalPixel;
constexpr int kGridColumns = 4;
constexpr int kGridRows = 3;
constexpr int kTimingRepeats = 5;
constexpr unsigned char kSentinel = 0xA5;

struct Scenario
{
    const char* name;
    int tilePixels;
    int tileTwips;
};

struct Measurement
{
    std::uint64_t rawBytesPerTile = 0;
    std::uint64_t rawBytesPerPass = 0;
    std::uint64_t minMicroseconds = 0;
    std::uint64_t medianMicroseconds = 0;
    std::uint64_t checksum = 0;
};

int fail(const std::string& message)
{
    std::cerr << "render_transfer_probe_error=" << message << '\n';
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

std::uint64_t checkedProduct(std::uint64_t left, std::uint64_t right, const char* context)
{
    if (left != 0 && right > std::numeric_limits<std::uint64_t>::max() / left)
        throw std::runtime_error(std::string("byte-count overflow for ") + context);
    return left * right;
}

std::uint64_t rawRgbaBytes(std::uint64_t width, std::uint64_t height)
{
    return checkedProduct(checkedProduct(width, height, "pixel area"), kBytesPerPixel, "RGBA bytes");
}

std::uint64_t ceilDiv(std::uint64_t value, std::uint64_t divisor)
{
    return value / divisor + (value % divisor == 0 ? 0 : 1);
}

Measurement measureScenario(lok::Document& document, const Scenario& scenario)
{
    if (scenario.tilePixels <= 0 || scenario.tileTwips <= 0)
        throw std::runtime_error("render scenario has non-positive dimensions");

    Measurement measurement;
    measurement.rawBytesPerTile = rawRgbaBytes(
        static_cast<std::uint64_t>(scenario.tilePixels),
        static_cast<std::uint64_t>(scenario.tilePixels));
    measurement.rawBytesPerPass = checkedProduct(
        measurement.rawBytesPerTile,
        static_cast<std::uint64_t>(kGridColumns * kGridRows),
        "grid pass bytes");

    std::vector<unsigned char> pixels(
        static_cast<std::size_t>(measurement.rawBytesPerTile), kSentinel);

    // Warm one tile and prove that the caller-owned buffer is actually written before timing.
    document.paintTile(
        pixels.data(),
        scenario.tilePixels,
        scenario.tilePixels,
        0,
        0,
        scenario.tileTwips,
        scenario.tileTwips);
    if (std::all_of(
            pixels.cbegin(), pixels.cend(), [](unsigned char byte) { return byte == kSentinel; }))
    {
        throw std::runtime_error(std::string("paintTile did not write ") + scenario.name + " buffer");
    }

    std::vector<std::uint64_t> durations;
    durations.reserve(kTimingRepeats);
    std::uint64_t checksum = 14695981039346656037ULL;
    for (int repeat = 0; repeat < kTimingRepeats; ++repeat)
    {
        const auto started = std::chrono::steady_clock::now();
        for (int row = 0; row < kGridRows; ++row)
        {
            for (int column = 0; column < kGridColumns; ++column)
            {
                document.paintTile(
                    pixels.data(),
                    scenario.tilePixels,
                    scenario.tilePixels,
                    column * scenario.tileTwips,
                    row * scenario.tileTwips,
                    scenario.tileTwips,
                    scenario.tileTwips);

                // Sample the completed caller-owned buffer outside LibreOffice. This keeps the
                // workload observable without adding a full-buffer hash to the timed hot path.
                const std::size_t sample = static_cast<std::size_t>(
                    (repeat * 17 + row * 7 + column * 3) % static_cast<int>(pixels.size()));
                checksum ^= pixels[sample];
                checksum *= 1099511628211ULL;
            }
        }
        const auto finished = std::chrono::steady_clock::now();
        const auto elapsed = std::chrono::duration_cast<std::chrono::microseconds>(
            finished - started);
        durations.push_back(static_cast<std::uint64_t>(elapsed.count()));
    }

    std::sort(durations.begin(), durations.end());
    measurement.minMicroseconds = durations.front();
    measurement.medianMicroseconds = durations[durations.size() / 2];
    measurement.checksum = checksum;
    return measurement;
}

void printScenario(const Scenario& scenario, const Measurement& measurement)
{
    std::cout << "render_transfer_" << scenario.name << "_tile_pixels="
              << scenario.tilePixels << 'x' << scenario.tilePixels << '\n';
    std::cout << "render_transfer_" << scenario.name << "_tile_twips="
              << scenario.tileTwips << 'x' << scenario.tileTwips << '\n';
    std::cout << "render_transfer_" << scenario.name << "_raw_bytes_per_tile="
              << measurement.rawBytesPerTile << '\n';
    std::cout << "render_transfer_" << scenario.name << "_grid_tiles="
              << (kGridColumns * kGridRows) << '\n';
    std::cout << "render_transfer_" << scenario.name << "_raw_bytes_per_grid_pass="
              << measurement.rawBytesPerPass << '\n';
    std::cout << "render_transfer_" << scenario.name << "_grid_min_us="
              << measurement.minMicroseconds << '\n';
    std::cout << "render_transfer_" << scenario.name << "_grid_p50_us="
              << measurement.medianMicroseconds << '\n';
    std::cout << "render_transfer_" << scenario.name << "_checksum="
              << measurement.checksum << '\n';
}
} // namespace

int main(int argc, char* argv[])
{
    if (argc != 4)
    {
        std::cerr << "usage: render-transfer-probe INSTALL_PATH PROFILE_URL INPUT.docx\n";
        return 2;
    }

    const char* installPath = argv[1];
    const char* profileUrl = argv[2];
    const char* inputPath = argv[3];

    try
    {
        std::unique_ptr<lok::Office> office(lok::lok_cpp_init(installPath, profileUrl));
        if (!office)
            return fail("could not initialise LibreOfficeKit");

        std::unique_ptr<lok::Document> document(office->documentLoad(inputPath));
        if (!document)
            return fail("could not load input DOCX: " + takeError(*office));
        if (document->getDocumentType() != LOK_DOCTYPE_TEXT)
            return fail("input fixture is not a Writer/text document");

        document->initializeForRendering();
        long widthTwips = 0;
        long heightTwips = 0;
        document->getDocumentSize(&widthTwips, &heightTwips);
        if (widthTwips <= 0 || heightTwips <= 0)
            return fail("Writer returned invalid document dimensions");

        const int tileMode = document->getTileMode();
        if (tileMode != LOK_TILEMODE_RGBA && tileMode != LOK_TILEMODE_BGRA)
            return fail("Writer returned an unsupported tile pixel mode");

        const Scenario oneX{"1x", kLogicalTilePixels, kLogicalTileTwips};
        const Scenario twoX{"2x", kLogicalTilePixels * 2, kLogicalTileTwips};
        const Measurement oneXMeasurement = measureScenario(*document, oneX);
        const Measurement twoXMeasurement = measureScenario(*document, twoX);

        const std::uint64_t pageWidth1x = ceilDiv(
            static_cast<std::uint64_t>(widthTwips), kTwipsPerLogicalPixel);
        const std::uint64_t pageHeight1x = ceilDiv(
            static_cast<std::uint64_t>(heightTwips), kTwipsPerLogicalPixel);
        const std::uint64_t pageWidth2x = checkedProduct(pageWidth1x, 2, "2x page width");
        const std::uint64_t pageHeight2x = checkedProduct(pageHeight1x, 2, "2x page height");

        std::cout << "render_transfer_document_twips=" << widthTwips << 'x' << heightTwips << '\n';
        std::cout << "render_transfer_pixel_mode="
                  << (tileMode == LOK_TILEMODE_RGBA ? "rgba" : "bgra") << '\n';
        std::cout << "render_transfer_bytes_per_pixel=" << kBytesPerPixel << '\n';
        std::cout << "render_transfer_logical_viewport_pixels="
                  << kLogicalViewportWidthPx << 'x' << kLogicalViewportHeightPx << '\n';
        std::cout << "render_transfer_logical_viewport_twips="
                  << (kLogicalViewportWidthPx * kTwipsPerLogicalPixel) << 'x'
                  << (kLogicalViewportHeightPx * kTwipsPerLogicalPixel) << '\n';
        std::cout << "render_transfer_viewport_1x_raw_bytes="
                  << rawRgbaBytes(kLogicalViewportWidthPx, kLogicalViewportHeightPx) << '\n';
        std::cout << "render_transfer_viewport_2x_raw_bytes="
                  << rawRgbaBytes(
                         static_cast<std::uint64_t>(kLogicalViewportWidthPx) * 2,
                         static_cast<std::uint64_t>(kLogicalViewportHeightPx) * 2)
                  << '\n';
        std::cout << "render_transfer_page_1x_pixels=" << pageWidth1x << 'x' << pageHeight1x << '\n';
        std::cout << "render_transfer_page_1x_raw_bytes="
                  << rawRgbaBytes(pageWidth1x, pageHeight1x) << '\n';
        std::cout << "render_transfer_page_2x_pixels=" << pageWidth2x << 'x' << pageHeight2x << '\n';
        std::cout << "render_transfer_page_2x_raw_bytes="
                  << rawRgbaBytes(pageWidth2x, pageHeight2x) << '\n';
        printScenario(oneX, oneXMeasurement);
        printScenario(twoX, twoXMeasurement);
        std::cout << "render_transfer_timing_policy=observational-no-ci-threshold\n";
        std::cout << "render_transfer_probe_status=observed\n";
        std::cout.flush();

        // Keep the same R0A process-reclamation discipline as the other direct native probes:
        // owned objects are destroyed before process-level exit skips only the pinned unsafe
        // LibreOffice global-finalizer phase.
        document.reset();
        office.reset();
        std::_Exit(0);
    }
    catch (const std::exception& error)
    {
        return fail(std::string("render-transfer exception: ") + error.what());
    }
}
