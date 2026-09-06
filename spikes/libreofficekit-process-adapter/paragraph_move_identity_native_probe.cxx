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

    void* qualificationLibrary = nullptr;
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

        std::string semanticError;
        auto semanticView = r0a::WriterSemanticView::acquire(semanticError);
        if (!semanticView)
            return fail("could not acquire same-authority Writer semantic view: " + semanticError);

        const auto before = semanticView->identityProbeParagraphs(
            kMaxParagraphs, kMaxIdentityBytes);
        const auto beforeRepeat = semanticView->identityProbeParagraphs(
            kMaxParagraphs, kMaxIdentityBytes);
        if (!sameSnapshot(before, beforeRepeat))
            return fail("baseline identity projection is not repeatable");
        if (!hasExpectedTexts(before, kExpectedBefore))
            return fail("baseline identity projection does not match deterministic fixture");
        if (!hasUniqueProbeTokens(before))
            return fail("baseline identity projection contains invalid or duplicate tokens");

        const auto beforeSemantic = semanticView->paragraphs(
            kMaxParagraphs, kMaxSemanticBytes);
        if (!hasExpectedTexts(beforeSemantic, kExpectedBefore))
            return fail("normal semantic projection disagrees with move baseline");

        qualificationLibrary = dlopen(modulePath, RTLD_NOW | RTLD_LOCAL);
        if (qualificationLibrary == nullptr)
        {
            const char* loaderError = dlerror();
            return fail(
                std::string("could not reopen qualification module: ")
                + (loaderError == nullptr ? "unknown dynamic-loader error" : loaderError));
        }

        const auto moveFirstParagraphDown = loadFunction<r0a::WriterMoveFirstParagraphDownFn>(
            qualificationLibrary,
            "r0a_writer_semantics_move_first_paragraph_down");
        std::array<char, kErrorBytes> moveError{};
        const int moveStatus = moveFirstParagraphDown(moveError.data(), moveError.size());
        if (moveStatus != r0a::kWriterSemanticStatusOk)
        {
            return fail(
                moveError[0] == '\0'
                    ? "Writer-native paragraph move failed without an error message"
                    : std::string(moveError.data()));
        }

        // Successful dispatch is deliberately insufficient evidence. Require the
        // same live semantic view to observe the exact reordered paragraph text.
        const auto after = semanticView->identityProbeParagraphs(
            kMaxParagraphs, kMaxIdentityBytes);
        const auto afterRepeat = semanticView->identityProbeParagraphs(
            kMaxParagraphs, kMaxIdentityBytes);
        if (!sameSnapshot(after, afterRepeat))
            return fail("identity projection is not repeatable after Writer-native move");
        if (!hasExpectedTexts(after, kExpectedAfter))
            return fail("Writer-native move did not produce exact P1,P0,P2 paragraph order");
        if (!hasUniqueProbeTokens(after))
            return fail("post-move identity projection contains invalid or duplicate tokens");

        const auto afterSemantic = semanticView->paragraphs(
            kMaxParagraphs, kMaxSemanticBytes);
        if (!hasExpectedTexts(afterSemantic, kExpectedAfter))
            return fail("normal semantic projection disagrees with post-move identity projection");

        std::cout << "native_move_tokens_before=" << formatTokens(before) << '\n';
        std::cout << "native_move_tokens_after=" << formatTokens(after) << '\n';
        std::cout << "native_move_identity_relation=" << formatRelation(before, after) << '\n';
        std::cout << "native_move_probe_repeatable=ok\n";
        std::cout << "native_move_semantic_order=P1-P0-P2\n";
        std::cout << "native_move_identity_status=observed\n";
        std::cout.flush();

        // Preserve the already-qualified shutdown ordering: release the semantic
        // view first, then the extra dlopen reference, then document and Office.
        semanticView.reset();
        dlclose(qualificationLibrary);
        qualificationLibrary = nullptr;
        document.reset();
        office.reset();
        return 0;
    }
    catch (const std::exception& error)
    {
        if (qualificationLibrary != nullptr)
            dlclose(qualificationLibrary);
        return fail(error.what());
    }
}
