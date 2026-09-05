#define LOK_USE_UNSTABLE_API 1

#include <LibreOfficeKit/LibreOfficeKit.hxx>

#include "writer_semantics_24_2.hxx"

#include <algorithm>
#include <array>
#include <cstdint>
#include <cstring>
#include <iostream>
#include <limits>
#include <memory>
#include <string>
#include <vector>

namespace
{
constexpr std::array<unsigned char, 4> kMagic{'D', 'E', 'T', 'R'};
constexpr std::uint16_t kFrameVersion = 1;
constexpr std::size_t kHeaderBytes = 20;
constexpr std::size_t kMaxPayloadBytes = 1024;
constexpr unsigned char kRequestKind = 1;
constexpr unsigned char kResponseKind = 2;
constexpr unsigned char kFlags = 0;

constexpr unsigned char kStatusOk = 0;
constexpr unsigned char kStatusInvalidRequest = 1;
constexpr unsigned char kStatusLoadFailed = 2;
constexpr unsigned char kStatusIncompatibleDocument = 3;
constexpr unsigned char kStatusEngineState = 4;
constexpr unsigned char kStatusLimitExceeded = 5;

constexpr unsigned char kCommandEngineInfo = 1;
constexpr unsigned char kCommandOpen = 2;
constexpr unsigned char kCommandClose = 3;
constexpr unsigned char kCommandShutdown = 4;
constexpr unsigned char kCommandSemanticSnapshot = 5;
constexpr unsigned char kCommandInsertPrefix = 6;

constexpr unsigned char kSemanticProjectionVersion = 2;
constexpr std::size_t kSemanticResponseFixedBytes = 13;
constexpr std::size_t kSemanticParagraphLengthBytes = 2;
static_assert(kMaxPayloadBytes > kSemanticResponseFixedBytes);
constexpr std::size_t kMaxSemanticEncodedParagraphBytes =
    kMaxPayloadBytes - kSemanticResponseFixedBytes;
constexpr std::size_t kMaxSemanticParagraphs =
    kMaxSemanticEncodedParagraphBytes / kSemanticParagraphLengthBytes;
constexpr std::size_t kMaxPrefixBytes = 256;

struct Frame
{
    std::uint64_t requestId = 0;
    std::vector<unsigned char> payload;
};

enum class ReadFrameResult
{
    Ok,
    CleanEof,
    Error,
};

std::uint16_t readU16(const unsigned char* bytes)
{
    return static_cast<std::uint16_t>(bytes[0])
           | (static_cast<std::uint16_t>(bytes[1]) << 8U);
}

std::uint32_t readU32(const unsigned char* bytes)
{
    std::uint32_t value = 0;
    for (unsigned int index = 0; index < 4; ++index)
        value |= static_cast<std::uint32_t>(bytes[index]) << (8U * index);
    return value;
}

std::uint64_t readU64(const unsigned char* bytes)
{
    std::uint64_t value = 0;
    for (unsigned int index = 0; index < 8; ++index)
        value |= static_cast<std::uint64_t>(bytes[index]) << (8U * index);
    return value;
}

void writeU16(unsigned char* bytes, std::uint16_t value)
{
    for (unsigned int index = 0; index < 2; ++index)
        bytes[index] = static_cast<unsigned char>((value >> (8U * index)) & 0xffU);
}

void writeU32(unsigned char* bytes, std::uint32_t value)
{
    for (unsigned int index = 0; index < 4; ++index)
        bytes[index] = static_cast<unsigned char>((value >> (8U * index)) & 0xffU);
}

void writeU64(unsigned char* bytes, std::uint64_t value)
{
    for (unsigned int index = 0; index < 8; ++index)
        bytes[index] = static_cast<unsigned char>((value >> (8U * index)) & 0xffU);
}

void appendU16(std::vector<unsigned char>& bytes, std::uint16_t value)
{
    const std::size_t offset = bytes.size();
    bytes.resize(offset + 2);
    writeU16(bytes.data() + offset, value);
}

void appendU64(std::vector<unsigned char>& bytes, std::uint64_t value)
{
    const std::size_t offset = bytes.size();
    bytes.resize(offset + 8);
    writeU64(bytes.data() + offset, value);
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

std::vector<unsigned char> errorPayload(
    unsigned char status, unsigned char command, const std::string& message)
{
    constexpr std::size_t kMaxErrorTextBytes = 512;
    std::vector<unsigned char> payload;
    const std::size_t textBytes = std::min(message.size(), kMaxErrorTextBytes);
    payload.reserve(2 + textBytes);
    payload.push_back(status);
    payload.push_back(command);
    payload.insert(payload.end(), message.begin(), message.begin() + static_cast<std::ptrdiff_t>(textBytes));
    return payload;
}

ReadFrameResult readFrame(Frame& frame)
{
    std::array<unsigned char, kHeaderBytes> header{};
    std::cin.read(reinterpret_cast<char*>(header.data()), static_cast<std::streamsize>(header.size()));
    const std::streamsize headerBytes = std::cin.gcount();
    if (headerBytes == 0 && std::cin.eof())
        return ReadFrameResult::CleanEof;
    if (headerBytes != static_cast<std::streamsize>(header.size()))
    {
        std::cerr << "native_adapter_transport_error=truncated_header bytes=" << headerBytes << '\n';
        return ReadFrameResult::Error;
    }

    if (!std::equal(kMagic.begin(), kMagic.end(), header.begin()))
    {
        std::cerr << "native_adapter_transport_error=bad_magic\n";
        return ReadFrameResult::Error;
    }

    const std::uint16_t version = readU16(header.data() + 4);
    if (version != kFrameVersion)
    {
        std::cerr << "native_adapter_transport_error=unsupported_version value=" << version << '\n';
        return ReadFrameResult::Error;
    }
    if (header[6] != kRequestKind)
    {
        std::cerr << "native_adapter_transport_error=unexpected_frame_kind value="
                  << static_cast<unsigned int>(header[6]) << '\n';
        return ReadFrameResult::Error;
    }
    if (header[7] != kFlags)
    {
        std::cerr << "native_adapter_transport_error=unsupported_flags value="
                  << static_cast<unsigned int>(header[7]) << '\n';
        return ReadFrameResult::Error;
    }

    frame.requestId = readU64(header.data() + 8);
    const std::uint32_t payloadBytes = readU32(header.data() + 16);
    if (static_cast<std::size_t>(payloadBytes) > kMaxPayloadBytes)
    {
        std::cerr << "native_adapter_transport_error=payload_too_large bytes=" << payloadBytes << '\n';
        return ReadFrameResult::Error;
    }

    frame.payload.assign(payloadBytes, 0);
    if (payloadBytes != 0)
    {
        std::cin.read(
            reinterpret_cast<char*>(frame.payload.data()),
            static_cast<std::streamsize>(frame.payload.size()));
        const std::streamsize received = std::cin.gcount();
        if (received != static_cast<std::streamsize>(frame.payload.size()))
        {
            std::cerr << "native_adapter_transport_error=truncated_payload expected="
                      << payloadBytes << " received=" << received << '\n';
            return ReadFrameResult::Error;
        }
    }

    return ReadFrameResult::Ok;
}

bool writeFrame(std::uint64_t requestId, const std::vector<unsigned char>& payload)
{
    if (payload.size() > kMaxPayloadBytes)
    {
        std::cerr << "native_adapter_internal_error=response_too_large\n";
        return false;
    }

    std::array<unsigned char, kHeaderBytes> header{};
    std::copy(kMagic.begin(), kMagic.end(), header.begin());
    writeU16(header.data() + 4, kFrameVersion);
    header[6] = kResponseKind;
    header[7] = kFlags;
    writeU64(header.data() + 8, requestId);
    writeU32(header.data() + 16, static_cast<std::uint32_t>(payload.size()));

    std::cout.write(reinterpret_cast<const char*>(header.data()), static_cast<std::streamsize>(header.size()));
    if (!payload.empty())
    {
        std::cout.write(
            reinterpret_cast<const char*>(payload.data()),
            static_cast<std::streamsize>(payload.size()));
    }
    std::cout.flush();
    return static_cast<bool>(std::cout);
}

bool containsNul(const std::vector<unsigned char>& payload, std::size_t offset)
{
    return std::find(payload.begin() + static_cast<std::ptrdiff_t>(offset), payload.end(), 0)
           != payload.end();
}

std::vector<unsigned char> engineInfo(lok::Office& office)
{
    char* raw = office.getVersionInfo();
    if (raw == nullptr)
        return errorPayload(kStatusEngineState, kCommandEngineInfo, "no version information");

    const std::string version(raw);
    office.freeError(raw);
    std::vector<unsigned char> payload{kStatusOk, kCommandEngineInfo};
    payload.insert(payload.end(), version.begin(), version.end());
    return payload;
}

std::vector<unsigned char> openDocument(
    lok::Office& office,
    std::unique_ptr<lok::Document>& document,
    std::unique_ptr<r0a::WriterSemanticView>& semanticView,
    std::uint64_t& documentRevision,
    const std::vector<unsigned char>& request)
{
    if (document)
        return errorPayload(kStatusEngineState, kCommandOpen, "a document is already open");
    if (request.size() <= 1 || containsNul(request, 1))
        return errorPayload(kStatusInvalidRequest, kCommandOpen, "invalid document path");

    const std::string path(
        reinterpret_cast<const char*>(request.data() + 1),
        request.size() - 1);
    std::unique_ptr<lok::Document> candidate(office.documentLoad(path.c_str()));
    if (!candidate)
        return errorPayload(kStatusLoadFailed, kCommandOpen, takeError(office));

    if (candidate->getDocumentType() != LOK_DOCTYPE_TEXT)
        return errorPayload(kStatusIncompatibleDocument, kCommandOpen, "document is not Writer/text");

    candidate->initializeForRendering();
    long width = 0;
    long height = 0;
    candidate->getDocumentSize(&width, &height);
    if (width <= 0 || height <= 0)
        return errorPayload(kStatusEngineState, kCommandOpen, "invalid Writer document dimensions");

    std::string semanticError;
    auto candidateSemanticView = r0a::WriterSemanticView::acquire(semanticError);
    if (!candidateSemanticView)
        return errorPayload(kStatusEngineState, kCommandOpen, semanticError);

    document = std::move(candidate);
    semanticView = std::move(candidateSemanticView);
    documentRevision = 0;

    std::vector<unsigned char> response{kStatusOk, kCommandOpen, 1};
    appendU64(response, static_cast<std::uint64_t>(width));
    appendU64(response, static_cast<std::uint64_t>(height));
    return response;
}

std::vector<unsigned char> semanticSnapshot(
    const r0a::WriterSemanticView* semanticView,
    std::uint64_t documentRevision)
{
    if (semanticView == nullptr)
        return errorPayload(kStatusEngineState, kCommandSemanticSnapshot, "no Writer document is open");

    const r0a::ParagraphSnapshot snapshot = semanticView->paragraphs(
        kMaxSemanticParagraphs,
        kMaxSemanticEncodedParagraphBytes);
    if (snapshot.status == r0a::SemanticReadStatus::LimitExceeded)
    {
        return errorPayload(
            kStatusLimitExceeded,
            kCommandSemanticSnapshot,
            snapshot.error);
    }
    if (snapshot.status == r0a::SemanticReadStatus::Error)
        return errorPayload(kStatusEngineState, kCommandSemanticSnapshot, snapshot.error);

    const std::vector<std::string>& paragraphs = snapshot.paragraphs;
    if (paragraphs.size() > 0xffffU)
        return errorPayload(kStatusLimitExceeded, kCommandSemanticSnapshot, "too many paragraphs");

    std::vector<unsigned char> response{
        kStatusOk,
        kCommandSemanticSnapshot,
        kSemanticProjectionVersion,
    };
    appendU64(response, documentRevision);
    appendU16(response, static_cast<std::uint16_t>(paragraphs.size()));

    for (const std::string& paragraph : paragraphs)
    {
        if (paragraph.size() > 0xffffU
            || response.size() > kMaxPayloadBytes - kSemanticParagraphLengthBytes
            || paragraph.size()
                   > kMaxPayloadBytes - kSemanticParagraphLengthBytes - response.size())
        {
            return errorPayload(
                kStatusLimitExceeded,
                kCommandSemanticSnapshot,
                "semantic snapshot exceeds R0A payload bound");
        }
        appendU16(response, static_cast<std::uint16_t>(paragraph.size()));
        response.insert(response.end(), paragraph.begin(), paragraph.end());
    }
    return response;
}

std::vector<unsigned char> insertPrefix(
    lok::Document* document,
    std::uint64_t& documentRevision,
    const std::vector<unsigned char>& request)
{
    if (document == nullptr)
        return errorPayload(kStatusEngineState, kCommandInsertPrefix, "no Writer document is open");
    if (request.size() <= 1 || request.size() - 1 > kMaxPrefixBytes || containsNul(request, 1))
        return errorPayload(kStatusInvalidRequest, kCommandInsertPrefix, "invalid prefix text");
    if (documentRevision == std::numeric_limits<std::uint64_t>::max())
        return errorPayload(kStatusEngineState, kCommandInsertPrefix, "document revision exhausted");

    document->postUnoCommand(".uno:GoToStartOfDoc", nullptr, false);
    if (!document->paste(
            "text/plain;charset=utf-8",
            reinterpret_cast<const char*>(request.data() + 1),
            request.size() - 1))
    {
        return errorPayload(kStatusEngineState, kCommandInsertPrefix, "LibreOfficeKit prefix edit failed");
    }
    ++documentRevision;
    return {kStatusOk, kCommandInsertPrefix};
}
} // namespace

int main(int argc, char* argv[])
{
    if (argc != 3)
    {
        std::cerr << "usage: lok-process-adapter INSTALL_PATH PROFILE_URL\n";
        return 2;
    }

    try
    {
        std::unique_ptr<lok::Office> office(lok::lok_cpp_init(argv[1], argv[2]));
        if (!office)
        {
            std::cerr << "native_adapter_init_error=could_not_initialise_libreofficekit\n";
            return 3;
        }

        std::unique_ptr<lok::Document> document;
        std::unique_ptr<r0a::WriterSemanticView> semanticView;
        std::uint64_t documentRevision = 0;
        for (;;)
        {
            Frame frame;
            const ReadFrameResult result = readFrame(frame);
            if (result == ReadFrameResult::CleanEof)
                return 0;
            if (result == ReadFrameResult::Error)
                return 4;

            if (frame.payload.empty())
            {
                if (!writeFrame(
                        frame.requestId,
                        errorPayload(kStatusInvalidRequest, 0, "empty command")))
                    return 5;
                continue;
            }

            const unsigned char command = frame.payload[0];
            std::vector<unsigned char> response;
            bool shutdown = false;
            switch (command)
            {
                case kCommandEngineInfo:
                    if (frame.payload.size() != 1)
                        response = errorPayload(kStatusInvalidRequest, command, "invalid engine-info request");
                    else
                        response = engineInfo(*office);
                    break;
                case kCommandOpen:
                    response = openDocument(
                        *office,
                        document,
                        semanticView,
                        documentRevision,
                        frame.payload);
                    break;
                case kCommandClose:
                    if (frame.payload.size() != 1)
                        response = errorPayload(kStatusInvalidRequest, command, "invalid close request");
                    else
                    {
                        semanticView.reset();
                        document.reset();
                        documentRevision = 0;
                        response = {kStatusOk, kCommandClose};
                    }
                    break;
                case kCommandShutdown:
                    if (frame.payload.size() != 1)
                        response = errorPayload(kStatusInvalidRequest, command, "invalid shutdown request");
                    else
                    {
                        semanticView.reset();
                        document.reset();
                        documentRevision = 0;
                        response = {kStatusOk, kCommandShutdown};
                        shutdown = true;
                    }
                    break;
                case kCommandSemanticSnapshot:
                    if (frame.payload.size() != 1)
                        response = errorPayload(kStatusInvalidRequest, command, "invalid semantic-snapshot request");
                    else
                        response = semanticSnapshot(semanticView.get(), documentRevision);
                    break;
                case kCommandInsertPrefix:
                    response = insertPrefix(document.get(), documentRevision, frame.payload);
                    break;
                default:
                    response = errorPayload(kStatusInvalidRequest, command, "unknown R0A command");
                    break;
            }

            if (!writeFrame(frame.requestId, response))
                return 5;
            if (shutdown)
                return 0;
        }
    }
    catch (const std::exception& error)
    {
        std::cerr << "native_adapter_exception=" << error.what() << '\n';
        return 6;
    }
}
