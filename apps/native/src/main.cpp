// ─────────────────────────────────────────────────────────────────────────────
//  Mochi DAW · native shell — MINIMAL borderless preview
//
//  Mirrors the web AppShell + TransportBar layout shape with blank
//  placeholders so the very first native build can be validated end-to-end
//  (Skia + Yoga + Win32 + SphereKit) before any DAW behaviour is ported.
//
//  Web → native layout mapping:
//
//   ┌───────────────────────────────────────────────────────────────────┐
//   │ TransportBar (h = TRANSPORT_BAR_H, daw-sunken)                    │
//   ├──────────┬──────────────────────────────────────────┬─────────────┤
//   │          │ Arrangement (flex)                       │             │
//   │ Browser  ├──────────────────────────────────────────┤  Inspector  │
//   │ Panel    │ Mixer / bottom workspace (h = MIXER_H)   │  Panel      │
//   ├──────────┴──────────────────────────────────────────┴─────────────┤
//   │ StatusBar (h = STATUSBAR_H, daw-sunken)                           │
//   └───────────────────────────────────────────────────────────────────┘
//
//  All sections are blank colored regions for now.  The TransportBar is the
//  drag region for the borderless window and hosts the close/min/max chrome.
// ─────────────────────────────────────────────────────────────────────────────

#include <SPHXGraphicInterface.hpp>
#include <SPHXGraphicComponents.hpp>
#include <SPHXPainter.hpp>
// #include <BinaryResources.hpp>   // framework-bundled SVG icons & UI assets
#include <MochiResources.hpp>    // app-bundled fonts + DAW-specific assets
#include <windows.h>
#include <memory>
#include <string>

#include "MochiDawLayout.hpp"

using namespace SphereUI;
using namespace MochiDaw;

// ─── Palette ─────────────────────────────────────────────────────────────────
// Pulled from apps/web/src/theme.ts (daw-* tokens) so the native preview
// reads at a glance like the web shell.
namespace P {
    constexpr SPHXColor Bg            = SPHXColor::RGB( 15,  17,  21);
    constexpr SPHXColor Sunken        = SPHXColor::RGB( 11,  13,  17);
    constexpr SPHXColor Surface       = SPHXColor::RGB( 22,  25,  30);
    constexpr SPHXColor SurfaceHigh   = SPHXColor::RGB( 28,  32,  38);
    constexpr SPHXColor Border        = SPHXColor::RGB(255, 255, 255,  18);
    constexpr SPHXColor BorderHard    = SPHXColor::RGB(  0,   0,   0, 160);
    constexpr SPHXColor TextPrimary   = SPHXColor::RGB(232, 236, 242);
    constexpr SPHXColor TextSecondary = SPHXColor::RGB(160, 168, 178);
    constexpr SPHXColor TextFaint     = SPHXColor::RGB( 90,  96, 106);
    constexpr SPHXColor Accent        = SPHXColor::RGB( 72, 209, 204);
    constexpr SPHXColor ChromeHover   = SPHXColor::RGB(255, 255, 255,  18);
    constexpr SPHXColor CloseHover    = SPHXColor::RGB(180,  40,  40, 160);
}

// Single window handle for chrome / drag callbacks.
static IWindow* gWindow = nullptr;

// ─── Drag region — borderless title-bar surrogate ────────────────────────────
//
// Mirrors the web TransportBar's `drag-region-app` / `app-no-drag` split:
// empty space on the bar starts a window drag, while children (logo label,
// chrome buttons) consume their own clicks first.
class TransportBar : public FlexNode {
public:
    TransportBar() {
        style.setWidthFull();
        style.setHeight(TRANSPORT_BAR_H);
        style.setFlexShrink(0.f);
        style.flexDirection = FlexDirection::Row;
        style.setAlignItems(YGAlignCenter);
        style.setJustifyContent(YGJustifySpaceBetween);
        style.setPadding(0.f, 8.f);
        style.setGap(8.f);
        style.backgroundColor = P::Sunken;
    }

    void draw(SkCanvas* canvas) override {
        SPHXPainter(canvas).rect(frame, style.backgroundColor);
        // 1 px bottom rule, matches `border-b border-daw-border`.
        SkRect rule = SkRect::MakeXYWH(frame.fLeft, frame.fBottom - 1.f, frame.width(), 1.f);
        SPHXPainter(canvas).rect(rule, P::Border);
        drawChildren(canvas);
    }

    // Pass clicks to children first, drag on empty area.
    bool onMouseDown(float x, float y) override {
        for (auto it = children.rbegin(); it != children.rend(); ++it)
            if ((*it)->onMouseDown(x, y)) return true;
        if (hitTest(x, y) && gWindow) {
            gWindow->startDrag();
            return true;
        }
        return false;
    }
};

// ─── Chrome button (minimize / maximize / close) ─────────────────────────────
//
// Custom node so we can swap background per state and intercept mouse-down
// before the icon child swallows it (the icon has no onClick).
class ChromeBtn : public FlexNode {
    SPHXColor hoverBg;
    std::shared_ptr<IconNode> iconNode;

public:
    std::function<void()> action;

    ChromeBtn(const std::string& iconName, SPHXColor hBg) : hoverBg(hBg) {
        style.setWidth(28.f);
        style.setHeight(24.f);
        style.borderRadius = 4.f;
        style.setAlignItems(YGAlignCenter);
        style.setJustifyContent(YGJustifyCenter);
        enableHover = true;

        iconNode = std::make_shared<IconNode>();
        iconNode->setIcon(iconName);
        iconNode->color = P::TextSecondary;
        iconNode->strokeWidth = 1.6f;
        iconNode->style.setWidth(12.f);
        iconNode->style.setHeight(12.f);
        addChild(iconNode);
    }

    void draw(SkCanvas* canvas) override {
        SPHXColor bg = isHovered ? hoverBg : SPHXColor::transparent();
        SPHXPainter(canvas).roundRect(frame, style.borderRadius, bg);
        iconNode->color = isHovered ? P::TextPrimary : P::TextSecondary;
        drawChildren(canvas);
    }

    bool onMouseDown(float x, float y) override {
        if (hitTest(x, y)) {
            isPressed = true;
            if (action) action();
            return true;
        }
        return false;
    }
};

// ─── Builders ────────────────────────────────────────────────────────────────

static FlexNode::Ptr BuildTransportBar() {
    auto bar = std::make_shared<TransportBar>();

    // Left cluster — logo + project label.  Matches the web TransportBar's
    // logo+project section without yet wiring up the menu strip.
    auto left = FlexNode::Row();
    left->style.setAlignItems(YGAlignCenter);
    left->style.setGap(10.f);

    auto logoDot = std::make_shared<FlexNode>();
    logoDot->style.setWidth(10.f);
    logoDot->style.setHeight(10.f);
    logoDot->style.borderRadius = 5.f;
    logoDot->style.backgroundColor = P::Accent;
    left->addChild(logoDot);

    auto appLbl = std::make_shared<TextNode>("Mochi DAW");
    appLbl->fontSize = 12.f;
    appLbl->color    = P::TextPrimary;
    left->addChild(appLbl);

    auto sep = std::make_shared<FlexNode>();
    sep->style.setWidth(1.f);
    sep->style.setHeight(16.f);
    sep->style.backgroundColor = P::Border;
    left->addChild(sep);

    auto projLbl = std::make_shared<TextNode>("Untitled Project");
    projLbl->fontSize = 11.f;
    projLbl->color    = P::TextSecondary;
    left->addChild(projLbl);

    auto savedLbl = std::make_shared<TextNode>("NATIVE PREVIEW");
    savedLbl->fontSize = 8.f;
    savedLbl->color    = P::TextFaint;
    left->addChild(savedLbl);

    bar->addChild(left);

    // Right cluster — placeholder transport controls + chrome buttons.
    auto right = FlexNode::Row();
    right->style.setAlignItems(YGAlignCenter);
    right->style.setGap(4.f);

    auto transportPlaceholder = std::make_shared<TextNode>("PLAY · STOP · REC · LOOP   |   1.1.00   |   120 BPM   4/4");
    transportPlaceholder->fontSize = 10.f;
    transportPlaceholder->color    = P::TextFaint;
    right->addChild(transportPlaceholder);

    auto chromeSep = std::make_shared<FlexNode>();
    chromeSep->style.setWidth(8.f);
    right->addChild(chromeSep);

    auto minBtn = std::make_shared<ChromeBtn>("minus", P::ChromeHover);
    minBtn->action = [] { if (gWindow) gWindow->minimize(); };
    right->addChild(minBtn);

    auto maxBtn = std::make_shared<ChromeBtn>("maximize", P::ChromeHover);
    maxBtn->action = [] { if (gWindow) gWindow->maximize(); };
    right->addChild(maxBtn);

    auto closeBtn = std::make_shared<ChromeBtn>("x", P::CloseHover);
    closeBtn->action = [] { if (gWindow) gWindow->close(); };
    right->addChild(closeBtn);

    bar->addChild(right);
    return bar;
}

// Small helper — labelled placeholder panel so the blank shell stays
// readable while everything is empty.
static FlexNode::Ptr Placeholder(const std::string& label, SPHXColor bg, float flex,
                                 float fixedWidth = 0.f, float fixedHeight = 0.f) {
    auto col = FlexNode::Column();
    if (fixedWidth  > 0.f) col->style.setWidth(fixedWidth);
    if (fixedHeight > 0.f) col->style.setHeight(fixedHeight);
    if (flex        > 0.f) col->style.setFlex(flex);
    col->style.setAlignItems(YGAlignCenter);
    col->style.setJustifyContent(YGJustifyCenter);
    col->style.backgroundColor = bg;

    auto lbl = std::make_shared<TextNode>(label);
    lbl->fontSize = 11.f;
    lbl->color    = P::TextFaint;
    lbl->textAlign = TextAlign::Center;
    col->addChild(lbl);
    return col;
}

static FlexNode::Ptr BuildStatusBar() {
    auto bar = FlexNode::Row();
    bar->style.setWidthFull();
    bar->style.setHeight(STATUSBAR_H);
    bar->style.setFlexShrink(0.f);
    bar->style.setAlignItems(YGAlignCenter);
    bar->style.setJustifyContent(YGJustifySpaceBetween);
    bar->style.setPadding(0.f, 10.f);
    bar->style.backgroundColor = P::Sunken;

    auto left = std::make_shared<TextNode>("Mochi DAW · native preview");
    left->fontSize = 9.5f;
    left->color    = P::TextFaint;
    bar->addChild(left);

    auto right = std::make_shared<TextNode>("SphereEngine · Skia / D3D12 / Yoga");
    right->fontSize = 9.5f;
    right->color    = P::TextFaint;
    bar->addChild(right);

    return bar;
}

static FlexNode::Ptr CreateUI() {
    // Theme tokens reused by SphereKit components.
    Theme::Background    = P::Bg;
    Theme::Accent        = P::Accent;
    Theme::TextPrimary   = P::TextPrimary;
    Theme::TextSecondary = P::TextSecondary;
    Theme::BorderRadius  = 6.f;

    auto root = FlexNode::Column();
    root->style.setWidthFull();
    root->style.setFlex(1.f);
    root->style.backgroundColor = P::Bg;
    root->style.borderRadius    = 10.f;
    root->style.overflowHidden  = true;

    // ── Transport bar ────────────────────────────────────────────────────────
    root->addChild(BuildTransportBar());

    // ── Body: Browser | (Arrangement / Mixer) | Inspector ───────────────────
    auto body = FlexNode::Row();
    body->style.setWidthFull();
    body->style.setFlex(1.f);
    body->style.setGap(1.f);
    body->style.backgroundColor = P::BorderHard;

    body->addChild(Placeholder("BROWSER",  P::Surface, 0.f, BROWSER_W));

    auto centerCol = FlexNode::Column();
    centerCol->style.setFlex(1.f);
    centerCol->style.setGap(1.f);
    centerCol->style.backgroundColor = P::BorderHard;
    centerCol->addChild(Placeholder("ARRANGEMENT", P::Bg,      1.f));
    centerCol->addChild(Placeholder("MIXER",       P::Surface, 0.f, 0.f, MIXER_H));
    body->addChild(centerCol);

    body->addChild(Placeholder("INSPECTOR", P::Surface, 0.f, INSPECTOR_W));
    root->addChild(body);

    // ── Status bar ───────────────────────────────────────────────────────────
    root->addChild(BuildStatusBar());

    return root;
}

// ─── Entry point ─────────────────────────────────────────────────────────────
int WINAPI WinMain(HINSTANCE, HINSTANCE, LPSTR, int) {
    Application::getInstance().init();

    // Framework resources (SVG icons used by chrome buttons, scrollbars, etc.)
    // and our own resources (Inter font bytes + future DAW-specific glyphs).
    InitBinaryResources();
    // InitMochiResources();

    ThemeSwitcher::getInstance().setTheme(ThemeType::Dark);

    Window window("Mochi DAW", WINDOW_W, WINDOW_H);
    gWindow = &window;

    window.setDarkMode(true);
    window.setWindowMode(WindowMode::Borderless);
    window.setCornerPreference(CornerPreference::Round);
    window.setShadow(true);
    window.setMinSize(WINDOW_MIN_W, WINDOW_MIN_H);
    window.center();

    window.setRoot(CreateUI());
    window.run();
    return 0;
}
