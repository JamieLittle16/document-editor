#define LOK_USE_UNSTABLE_API 1

#include <LibreOfficeKit/LibreOfficeKit.hxx>

#include "writer_move_qualification_abi.hxx"
#include "writer_semantics_24_2.hxx"
#include "writer_semantics_module_abi.hxx"

#include <dlfcn.h>

#include <array>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <memory>
#include <set>
#include <sstream>
#include <stdexcept>
#include <string>

namespace
{
constexpr std::size_t kMaxParagraphs = 16;
constexpr std::size_t kMaxSemanticBytes = 4096;
constexpr std::size_t kMaxIdentityBytes = 4096;
constexpr std::size_t kErrorBytes = 512;
constexpr const char* kModulePathEnvironment = "R0A_WRITER_SEMANTICS_MODULE";

const std::array<std::string, 3> kExpectedBefore{
    "Document Editor LibreOfficeKit R0A probe",
    "This fixture is generated deterministically in CI.",
    "Stable semantic identity must be measured, not assumed.",
};

const std::array<std::string, 3> kExpectedAfter{
    kExpectedBefore[1],
    kExpectedBefore[0],
    kExpectedBefore[2],
};

int fail(const std::string& message)
{
    std::cerr << "native_move_probe_error=" << message << '\n';
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

template <typename Function>
Function loadFunction(void* library, const char* name)
{
    dlerror();
    void* symbol = dlsym(library, name);
    const char* error = dlerror();
    if (symbol == nullptr || error != nullptr)
    {
        throw std::runtime_error(
            std::string("qualification module is missing symbol ") + name + ": "
            + (error == nullptr ? "unknown dynamic-loader error" : error));
    }

    Function function = nullptr;
    static_assert(sizeof(function) == sizeof(symbol));
    std::memcpy(&function, &symbol, sizeof(function));
    return function;
}

bool sameSnapshot(
    const r0a::IdentityProbeSnapshot& left,
    const r0a::IdentityProbeSnapshot& right)
{
    if (left.status != right.status || left.paragraphs.size() != right.paragraphs.size())
        return false;

    for (std::size_t index = 0; index < left.paragraphs.size(); ++index)
    {
        if (left.paragraphs[index].probeToken != right.paragraphs[index].probeToken
            || left.paragraphs[index].text != right.paragraphs[index].text)
        {
            return false;
        }
    }
    return true;
}

template <std::size_t Size>
bool hasExpectedTexts(
    const r0a::IdentityProbeSnapshot& snapshot,
    const std::array<std::string, Size>& expected)
{
    if (snapshot.status != r0a::SemanticReadStatus::Ok
        || snapshot.paragraphs.size() != expected.size())
    {
        return false;
    }

    for (std::size_t index = 0; index < expected.size(); ++index)
    {
        if (snapshot.paragraphs[index].text != expected[index])
            return false;
    }
    return true;
}

template <std::size_t Size>
bool hasExpectedTexts(
    const r0a::ParagraphSnapshot& snapshot,
    const std::array<std::string, Size>& expected)
{
    if (snapshot.status != r0a::SemanticReadStatus::Ok
        || snapshot.paragraphs.size() != expected.size())
    {
        return false;
    }

    for (std::size_t index = 0; index < expected.size(); ++index)
    {
        if (snapshot.paragraphs[index] != expected[index])
            return false;
    }
    return true;
}

bool hasUniqueProbeTokens(const r0a::IdentityProbeSnapshot& snapshot)
{
    std::set<std::uint64_t> tokens;
    for (const auto& paragraph : snapshot.paragraphs)
    {
        if (paragraph.probeToken == 0 || !tokens.insert(paragraph.probeToken).second)
            return false;
    }
    return true;
}

std::string formatTokens(const r0a::IdentityProbeSnapshot& snapshot)
{
    std::ostringstream output;
    output << '(';
    for (std::size_t index = 0; index < snapshot.paragraphs.size(); ++index)
    {
        if (index != 0)
            output << ", ";
        output << snapshot.paragraphs[index].probeToken;
    }
    output << ')';
    return output.str();
}

std::string formatRelation(
    const r0a::IdentityProbeSnapshot& before,
    const r0a::IdentityProbeSnapshot& after)
{
    std::ostringstream output;
    for (std::size_t beforeIndex = 0; beforeIndex < before.paragraphs.size(); ++beforeIndex)
    {
        if (beforeIndex != 0)
            output << ';';
        output << beforeIndex << "->";

        bool found = false;
        for (std::size_t afterIndex = 0; afterIndex < after.paragraphs.size(); ++afterIndex)
        {
            if (before.paragraphs[beforeIndex].probeToken == after.paragraphs[afterIndex].probeToken)
            {
                output << afterIndex;
                found = true;
                break;
            }
        }
        if (!found)
            output << '-';
    }
    return output.str();
}

std::string paragraphLabel(const std::string& text)
{
    for (std::size_t index = 0; index < kExpectedBefore.size(); ++index)
    {
        if (text == kExpectedBefore[index])
            return "P" + std::to_string(index);
    }
    return "unknown";
}

std::string formatObservedOrder(const r0a::IdentityProbeSnapshot& snapshot)
{
    if (snapshot.status != r0a::SemanticReadStatus::Ok)
        return "read-error";

    std::ostringstream output;
    for (std::size_t index = 0; index < snapshot.paragraphs.size(); ++index)
    {
        if (index != 0)
            output << '-';
        output << paragraphLabel(snapshot.paragraphs[index].text);
    }
    return output.str();
}
} // namespace

int main(int argc, char* argv[])
{
    if (argc != 4)
    {
        std::cerr << "usage: paragraph-move-identity-probe INSTALL_PATH PROFILE_URL INPUT.docx\n";
        return 2;
    }

    const char* installPath = argv[1];
    const char* profileUrl = argv[2];
    const char* inputPath = argv[3];
    const char* modulePath = std::getenv(kModulePathEnvironment);
    if (modulePath == nullptr || modulePath[0] == '\0')
        return fail(std::string("missing ") + kModulePathEnvironment);

    std::unique_ptr<lok::Office> office;
    std::unique_ptr<lok::Document> document;
    std::unique_ptr<r0a::WriterSemanticView> semanticView;
    void* qualificationLibrary = nullptr;

    const auto finish = [&](int status, const std::string& message) {
        if (!message.empty())
            std::cerr << "native_move_probe_error=" << message << '\n';

        semanticView.reset();
        if (qualificationLibrary != nullptr)
        {
            dlclose(qualificationLibrary);
            qualificationLibrary = nullptr;
        }
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
            return fail("could not initialise LibreOfficeKit");

        document.reset(office->documentLoad(inputPath));
        if (!document)
            finish(1, "could not load input DOCX: " + takeError(*office));
        if (document->getDocumentType() != LOK_DOCTYPE_TEXT)
            finish(1, "input fixture is not a Writer/text document");
        document->initializeForRendering();

        std::string semanticError;
        semanticView = r0a::WriterSemanticView::acquire(semanticError);
        if (!semanticView)
            finish(1, "could not acquire same-authority Writer semantic view: " + semanticError);

        qualificationLibrary = dlopen(modulePath, RTLD_NOW | RTLD_LOCAL);
        if (qualificationLibrary == nullptr)
        {
            const char* loaderError = dlerror();
            finish(
                1,
                std::string("could not reopen qualification module: ")
                    + (loaderError == nullptr ? "unknown dynamic-loader error" : loaderError));
        }

        const auto prepareMoveContext = loadFunction<r0a::WriterPrepareParagraphMoveContextFn>(
            qualificationLibrary,
            "r0a_writer_semantics_prepare_paragraph_move_context");
        const auto moveFirstParagraphDown = loadFunction<r0a::WriterMoveFirstParagraphDownFn>(
            qualificationLibrary,
            "r0a_writer_semantics_move_first_paragraph_down");

        std::array<char, kErrorBytes> operationError{};
        const int prepareStatus = prepareMoveContext(operationError.data(), operationError.size());
        if (prepareStatus != r0a::kWriterSemanticStatusOk)
        {
            finish(
                1,
                operationError[0] == '\0'
                    ? "Writer paragraph move context preparation failed without an error message"
                    : std::string(operationError.data()));
        }

        // Setup formatting is deliberately outside the measured identity
        // relation. First prove it preserved the three semantic paragraphs, then
        // establish the move baseline and only afterwards invoke MoveDown.
        const auto preparedSemantic = semanticView->paragraphs(
            kMaxParagraphs, kMaxSemanticBytes);
        if (!hasExpectedTexts(preparedSemantic, kExpectedBefore))
            finish(1, "list-context preparation changed deterministic paragraph text");

        const auto before = semanticView->identityProbeParagraphs(
            kMaxParagraphs, kMaxIdentityBytes);
        const auto beforeRepeat = semanticView->identityProbeParagraphs(
            kMaxParagraphs, kMaxIdentityBytes);
        if (!sameSnapshot(before, beforeRepeat))
            finish(1, "prepared baseline identity projection is not repeatable");
        if (!hasExpectedTexts(before, kExpectedBefore))
            finish(1, "prepared baseline identity projection does not match deterministic fixture");
        if (!hasUniqueProbeTokens(before))
            finish(1, "prepared baseline identity projection contains invalid or duplicate tokens");

        operationError.fill('\0');
        const int moveStatus = moveFirstParagraphDown(operationError.data(), operationError.size());
        if (moveStatus != r0a::kWriterSemanticStatusOk)
        {
            finish(
                1,
                operationError[0] == '\0'
                    ? "Writer-native paragraph move failed without an error message"
                    : std::string(operationError.data()));
        }

        const auto after = semanticView->identityProbeParagraphs(
            kMaxParagraphs, kMaxIdentityBytes);
        const auto afterRepeat = semanticView->identityProbeParagraphs(
            kMaxParagraphs, kMaxIdentityBytes);

        std::cout << "native_move_tokens_before=" << formatTokens(before) << '\n';
        std::cout << "native_move_tokens_after=" << formatTokens(after) << '\n';
        std::cout << "native_move_identity_relation=" << formatRelation(before, after) << '\n';
        std::cout << "native_move_observed_order=" << formatObservedOrder(after) << '\n';
        std::cout.flush();

        if (!sameSnapshot(after, afterRepeat))
            finish(1, "identity projection is not repeatable after Writer-native move");
        if (!hasExpectedTexts(after, kExpectedAfter))
            finish(1, "Writer-native move did not produce exact P1,P0,P2 paragraph order");
        if (!hasUniqueProbeTokens(after))
            finish(1, "post-move identity projection contains invalid or duplicate tokens");

        const auto afterSemantic = semanticView->paragraphs(
            kMaxParagraphs, kMaxSemanticBytes);
        if (!hasExpectedTexts(afterSemantic, kExpectedAfter))
            finish(1, "normal semantic projection disagrees with post-move identity projection");

        std::cout << "native_move_context=list\n";
        std::cout << "native_move_probe_repeatable=ok\n";
        std::cout << "native_move_semantic_order=P1-P0-P2\n";
        std::cout << "native_move_identity_status=observed\n";
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
