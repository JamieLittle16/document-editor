#define LOK_USE_UNSTABLE_API 1

#include <LibreOfficeKit/LibreOfficeKit.hxx>
#include <LibreOfficeKit/LibreOfficeKitEnums.h>

#include "writer_semantics_24_2.hxx"

#include <algorithm>
#include <atomic>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <iostream>
#include <memory>
#include <mutex>
#include <sstream>
#include <string>
#include <thread>
#include <vector>

namespace
{
constexpr std::size_t kMaxParagraphs = 16;
constexpr std::size_t kMaxSemanticBytes = 4096;
constexpr int kCanvasWidth = 256;
constexpr int kCanvasHeight = 256;
constexpr char kEditMarker[] = "R0A_CALLBACK_EDIT_91C4";

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

std::string takeError(lok::Office& office)
{
    char* raw = office.getError();
    if (raw == nullptr)
        return "unknown LibreOfficeKit error";
    const std::string value(raw);
    office.freeError(raw);
    return value;
}

enum class DeliveryPhase : int
{
    Baseline = 0,
    MutationCall = 1,
    ReturnedBeforeRevision = 2,
    RevisionAdvanced = 3,
};

const char* phaseName(DeliveryPhase phase)
{
    switch (phase)
    {
        case DeliveryPhase::Baseline:
            return "baseline";
        case DeliveryPhase::MutationCall:
            return "mutation-call";
        case DeliveryPhase::ReturnedBeforeRevision:
            return "returned-before-revision";
        case DeliveryPhase::RevisionAdvanced:
            return "revision-advanced";
    }
    return "unknown";
}

struct CallbackEvent
{
    int type = 0;
    std::string payload;
    std::uint64_t hostRevision = 0;
    DeliveryPhase phase = DeliveryPhase::Baseline;
    bool onOwnerThread = false;
};

class CallbackRecorder
{
public:
    CallbackRecorder()
        : ownerThread_(std::this_thread::get_id())
    {
    }

    static void callback(int type, const char* payload, void* data)
    {
        if (data == nullptr)
            return;
        static_cast<CallbackRecorder*>(data)->record(type, payload == nullptr ? "" : payload);
    }

    void setPhase(DeliveryPhase phase) noexcept
    {
        phase_.store(static_cast<int>(phase), std::memory_order_release);
    }

    void setHostRevision(std::uint64_t revision) noexcept
    {
        hostRevision_.store(revision, std::memory_order_release);
    }

    void clear()
    {
        std::lock_guard<std::mutex> lock(mutex_);
        events_.clear();
    }

    std::vector<CallbackEvent> snapshot() const
    {
        std::lock_guard<std::mutex> lock(mutex_);
        return events_;
    }

private:
    void record(int type, const char* payload)
    {
        CallbackEvent event;
        event.type = type;
        event.payload = payload;
        event.hostRevision = hostRevision_.load(std::memory_order_acquire);
        event.phase = static_cast<DeliveryPhase>(phase_.load(std::memory_order_acquire));
        event.onOwnerThread = std::this_thread::get_id() == ownerThread_;

        std::lock_guard<std::mutex> lock(mutex_);
        events_.push_back(std::move(event));
    }

    const std::thread::id ownerThread_;
    std::atomic<std::uint64_t> hostRevision_{0};
    std::atomic<int> phase_{static_cast<int>(DeliveryPhase::Baseline)};
    mutable std::mutex mutex_;
    std::vector<CallbackEvent> events_;
};

std::uint64_t paintHash(lok::Document& document)
{
    long width = 0;
    long height = 0;
    document.getDocumentSize(&width, &height);
    if (width <= 0 || height <= 0)
        throw std::runtime_error("LibreOfficeKit returned invalid document dimensions");

    std::vector<unsigned char> pixels(
        static_cast<std::size_t>(kCanvasWidth) * static_cast<std::size_t>(kCanvasHeight) * 4U);
    document.paintTile(
        pixels.data(),
        kCanvasWidth,
        kCanvasHeight,
        0,
        0,
        static_cast<int>(width),
        static_cast<int>(height));
    return fnv1a64(pixels);
}

bool validParagraphSnapshot(const r0a::ParagraphSnapshot& snapshot)
{
    return snapshot.status == r0a::SemanticReadStatus::Ok && snapshot.paragraphs.size() == 3;
}

std::string firstInvalidationPayload(const std::vector<CallbackEvent>& events)
{
    for (const auto& event : events)
    {
        if (event.type == LOK_CALLBACK_INVALIDATE_TILES)
            return event.payload;
    }
    return "none";
}

void printObservations(const std::vector<CallbackEvent>& events)
{
    std::size_t invalidations = 0;
    std::size_t duringCall = 0;
    std::size_t returnedWindow = 0;
    std::size_t afterRevision = 0;
    std::size_t offOwnerThread = 0;
    const CallbackEvent* firstInvalidation = nullptr;

    for (const auto& event : events)
    {
        if (!event.onOwnerThread)
            ++offOwnerThread;
        if (event.type != LOK_CALLBACK_INVALIDATE_TILES)
            continue;

        ++invalidations;
        if (firstInvalidation == nullptr)
            firstInvalidation = &event;
        switch (event.phase)
        {
            case DeliveryPhase::MutationCall:
                ++duringCall;
                break;
            case DeliveryPhase::ReturnedBeforeRevision:
                ++returnedWindow;
                break;
            case DeliveryPhase::RevisionAdvanced:
                ++afterRevision;
                break;
            case DeliveryPhase::Baseline:
                break;
        }
    }

    std::cout << "native_callback_total_events=" << events.size() << '\n';
    std::cout << "native_callback_invalidate_tiles_count=" << invalidations << '\n';
    std::cout << "native_callback_invalidations_during_mutation_call=" << duringCall << '\n';
    std::cout << "native_callback_invalidations_after_return_before_revision=" << returnedWindow << '\n';
    std::cout << "native_callback_invalidations_after_revision=" << afterRevision << '\n';
    std::cout << "native_callback_off_owner_thread_events=" << offOwnerThread << '\n';
    std::cout << "native_callback_first_invalidation_payload=" << firstInvalidationPayload(events)
              << '\n';
    if (firstInvalidation == nullptr)
    {
        std::cout << "native_callback_first_invalidation_phase=none\n";
        std::cout << "native_callback_first_invalidation_host_revision=none\n";
    }
    else
    {
        std::cout << "native_callback_first_invalidation_phase="
                  << phaseName(firstInvalidation->phase) << '\n';
        std::cout << "native_callback_first_invalidation_host_revision="
                  << firstInvalidation->hostRevision << '\n';
    }
}
} // namespace

int main(int argc, char* argv[])
{
    if (argc != 4)
    {
        std::cerr << "usage: invalidation-revision-probe INSTALL_PATH PROFILE_URL INPUT.docx\n";
        return 2;
    }

    const char* installPath = argv[1];
    const char* profileUrl = argv[2];
    const char* inputPath = argv[3];

    std::unique_ptr<lok::Office> office;
    std::unique_ptr<lok::Document> document;
    std::unique_ptr<r0a::WriterSemanticView> semanticView;
    CallbackRecorder recorder;

    const auto finish = [&](int status, const std::string& message) {
        if (!message.empty())
            std::cerr << "native_callback_probe_error=" << message << '\n';
        if (document)
            document->registerCallback(nullptr, nullptr);
        semanticView.reset();
        document.reset();
        office.reset();
        std::cout.flush();
        std::cerr.flush();
        std::_Exit(status);
    };

    try
    {
        office.reset(lok::lok_cpp_init(installPath, profileUrl));
        if (!office)
            finish(1, "could not initialise LibreOfficeKit");

        document.reset(office->documentLoad(inputPath));
        if (!document)
            finish(1, "could not load input DOCX: " + takeError(*office));
        if (document->getDocumentType() != LOK_DOCTYPE_TEXT)
            finish(1, "input fixture is not a Writer document");
        document->initializeForRendering();

        std::string semanticError;
        semanticView = r0a::WriterSemanticView::acquire(semanticError);
        if (!semanticView)
            finish(1, "could not acquire Writer semantic view: " + semanticError);

        const auto before = semanticView->paragraphs(kMaxParagraphs, kMaxSemanticBytes);
        if (!validParagraphSnapshot(before))
            finish(1, "baseline semantic projection was not the deterministic three-paragraph fixture");

        const std::uint64_t beforeHash = paintHash(*document);
        document->registerCallback(&CallbackRecorder::callback, &recorder);

        // Registration/rendering may emit initial view state. We care only about callbacks that
        // overlap the verified edit below, so discard everything observed before the edit phase.
        std::this_thread::sleep_for(std::chrono::milliseconds(25));
        recorder.clear();
        recorder.setHostRevision(0);
        recorder.setPhase(DeliveryPhase::MutationCall);

        if (!document->paste(
                "text/plain;charset=utf-8", kEditMarker, sizeof(kEditMarker) - 1U))
        {
            finish(1, "LibreOfficeKit paste mutation failed: " + takeError(*office));
        }

        recorder.setPhase(DeliveryPhase::ReturnedBeforeRevision);
        // Expose any callback implementation that delivers from another thread only after the LOK
        // call returned. Production code must not contain this delay; it is qualification-only.
        std::this_thread::sleep_for(std::chrono::milliseconds(50));

        // Model the process adapter's authoritative mutation rule: advance the host revision only
        // after the engine operation has returned success.
        recorder.setHostRevision(1);
        recorder.setPhase(DeliveryPhase::RevisionAdvanced);

        const auto after = semanticView->paragraphs(kMaxParagraphs, kMaxSemanticBytes);
        if (!validParagraphSnapshot(after))
            finish(1, "post-edit semantic projection was invalid");
        if (after.paragraphs[0] != std::string(kEditMarker) + before.paragraphs[0])
            finish(1, "post-edit semantic projection did not contain the exact callback probe edit");
        if (after.paragraphs[1] != before.paragraphs[1]
            || after.paragraphs[2] != before.paragraphs[2])
        {
            finish(1, "callback probe edit changed an unexpected paragraph");
        }

        const std::uint64_t afterHash = paintHash(*document);
        std::this_thread::sleep_for(std::chrono::milliseconds(50));
        const auto events = recorder.snapshot();

        printObservations(events);
        std::cout << "native_callback_semantic_revision_progression=R0-R1\n";
        std::cout << "native_callback_semantic_edit_verified=ok\n";
        std::cout << "native_callback_render_hash_changed="
                  << (beforeHash == afterHash ? "no" : "yes") << '\n';
        std::cout << "native_callback_observation_status=observed\n";
        std::cout.flush();
        finish(0, "");
    }
    catch (const std::exception& error)
    {
        finish(1, error.what());
    }
    catch (...)
    {
        finish(1, "unknown native exception");
    }
}
