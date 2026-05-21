#include "sphere_daux_vst3_processor.h"

#include <array>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <memory>
#include <mutex>
#include <string>

#include "pluginterfaces/base/ipluginbase.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "pluginterfaces/vst/ivstparameterchanges.h"
#include "pluginterfaces/vst/ivstprocesscontext.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/vst/hosting/module.h"
#include "public.sdk/source/vst/utility/uid.h"

namespace {

constexpr const char* kVst3AudioModuleClass = "Audio Module Class";

bool looks_like_zero_class_id(const std::string& value) {
  if (value.empty()) return true;
  for (char c : value) {
    if (c != '0' && c != '-' && c != '{' && c != '}') return false;
  }
  return true;
}

VST3::Optional<VST3::UID> first_audio_module_uid(const VST3::Hosting::PluginFactory& factory) {
  for (const auto& info : factory.classInfos()) {
    if (info.category() != kVst3AudioModuleClass) continue;
    return VST3::Optional<VST3::UID>(info.ID());
  }
  return {};
}

} // namespace

// ── Parameter helper types (stack/member allocated — zero heap use) ──────────

/// One pending parameter change (id + normalized value 0..1).
struct PendingParam {
  Steinberg::Vst::ParamID    id{0};
  Steinberg::Vst::ParamValue value{0.0};
};

/// Minimal IParamValueQueue: single value at sample-offset 0.
/// No heap allocation — lives inside SimpleParamChanges::queues[].
struct SimpleParamValueQueue final : Steinberg::Vst::IParamValueQueue {
  Steinberg::Vst::ParamID    param_id{0};
  Steinberg::Vst::ParamValue param_value{0.0};

  Steinberg::tresult PLUGIN_API queryInterface(
      const Steinberg::TUID iid, void** obj) override {
    if (std::memcmp(iid, Steinberg::Vst::IParamValueQueue::iid,
                    sizeof(Steinberg::TUID)) == 0) {
      *obj = this;
      return Steinberg::kResultOk;
    }
    *obj = nullptr;
    return Steinberg::kNoInterface;
  }
  Steinberg::uint32 PLUGIN_API addRef()  override { return 1; }
  Steinberg::uint32 PLUGIN_API release() override { return 1; }

  Steinberg::Vst::ParamID PLUGIN_API getParameterId() override { return param_id; }
  Steinberg::int32        PLUGIN_API getPointCount()  override { return 1; }

  Steinberg::tresult PLUGIN_API getPoint(
      Steinberg::int32 index,
      Steinberg::int32& sample_offset,
      Steinberg::Vst::ParamValue& value) override {
    if (index != 0) return Steinberg::kResultFalse;
    sample_offset = 0;
    value = param_value;
    return Steinberg::kResultOk;
  }
  Steinberg::tresult PLUGIN_API addPoint(
      Steinberg::int32,
      Steinberg::Vst::ParamValue v,
      Steinberg::int32& idx) override {
    param_value = v;
    idx = 0;
    return Steinberg::kResultOk;
  }
};

/// Minimal IParameterChanges: fixed capacity, no dynamic allocation.
/// Reused each process call — reset() clears the count without freeing memory.
struct SimpleParamChanges final : Steinberg::Vst::IParameterChanges {
  static constexpr int kMaxQueues = 64;

  std::array<SimpleParamValueQueue, kMaxQueues> queues{};
  int count{0};

  Steinberg::tresult PLUGIN_API queryInterface(
      const Steinberg::TUID iid, void** obj) override {
    if (std::memcmp(iid, Steinberg::Vst::IParameterChanges::iid,
                    sizeof(Steinberg::TUID)) == 0) {
      *obj = this;
      return Steinberg::kResultOk;
    }
    *obj = nullptr;
    return Steinberg::kNoInterface;
  }
  Steinberg::uint32 PLUGIN_API addRef()  override { return 1; }
  Steinberg::uint32 PLUGIN_API release() override { return 1; }

  Steinberg::int32 PLUGIN_API getParameterCount() override { return count; }

  Steinberg::Vst::IParamValueQueue* PLUGIN_API getParameterData(
      Steinberg::int32 index) override {
    if (index < 0 || index >= count) return nullptr;
    return &queues[index];
  }

  Steinberg::Vst::IParamValueQueue* PLUGIN_API addParameterData(
      const Steinberg::Vst::ParamID& id,
      Steinberg::int32& index) override {
    if (count >= kMaxQueues) return nullptr;
    index = count;
    queues[count].param_id    = id;
    queues[count].param_value = 0.0;
    return &queues[count++];
  }

  void reset() { count = 0; }
};

// Forward declaration so ComponentHandlerImpl can hold a back-pointer.
struct SphereDauxVst3Processor;

/// IComponentHandler that captures performEdit() callbacks from the plugin GUI
/// and enqueues them for delivery to IAudioProcessor on the next process call.
struct ComponentHandlerImpl final : Steinberg::Vst::IComponentHandler {
  SphereDauxVst3Processor* owner{nullptr};

  Steinberg::tresult PLUGIN_API queryInterface(
      const Steinberg::TUID iid, void** obj) override {
    if (std::memcmp(iid, Steinberg::Vst::IComponentHandler::iid,
                    sizeof(Steinberg::TUID)) == 0) {
      *obj = this;
      return Steinberg::kResultOk;
    }
    *obj = nullptr;
    return Steinberg::kNoInterface;
  }
  Steinberg::uint32 PLUGIN_API addRef()  override { return 1; }
  Steinberg::uint32 PLUGIN_API release() override { return 1; }

  Steinberg::tresult PLUGIN_API beginEdit(Steinberg::Vst::ParamID) override {
    return Steinberg::kResultOk;
  }
  Steinberg::tresult PLUGIN_API endEdit(Steinberg::Vst::ParamID) override {
    return Steinberg::kResultOk;
  }
  Steinberg::tresult PLUGIN_API restartComponent(Steinberg::int32) override {
    return Steinberg::kResultOk;
  }

  // Defined below, after SphereDauxVst3Processor is complete.
  Steinberg::tresult PLUGIN_API performEdit(
      Steinberg::Vst::ParamID id,
      Steinberg::Vst::ParamValue value) override;
};

// ── Main processor struct ─────────────────────────────────────────────────────

struct SphereDauxVst3Processor {
  static constexpr int kMaxPending = 64;

  VST3::Hosting::Module::Ptr                        module;
  Steinberg::Vst::HostApplication                   host_context;
  Steinberg::IPtr<Steinberg::Vst::IComponent>       component;
  Steinberg::IPtr<Steinberg::Vst::IAudioProcessor>  processor;
  Steinberg::IPtr<Steinberg::Vst::IEditController>  controller;
  Steinberg::IPtr<Steinberg::Vst::IConnectionPoint> component_connection;
  Steinberg::IPtr<Steinberg::Vst::IConnectionPoint> controller_connection;
  bool controller_is_component{false};

  // Stereo single-sample I/O buffers
  Steinberg::Vst::SpeakerArrangement input_arrangement  = Steinberg::Vst::SpeakerArr::kStereo;
  Steinberg::Vst::SpeakerArrangement output_arrangement = Steinberg::Vst::SpeakerArr::kStereo;
  float  input_l{0.f}, input_r{0.f};
  float  output_l{0.f}, output_r{0.f};
  float* input_channels[2]  = {&input_l,  &input_r};
  float* output_channels[2] = {&output_l, &output_r};
  Steinberg::Vst::AudioBusBuffers input_bus{};
  Steinberg::Vst::AudioBusBuffers output_bus{};
  Steinberg::Vst::ProcessContext  process_context{};
  Steinberg::Vst::ProcessData     process_data{};
  bool processing{false};

  // Diagnostics
  unsigned long long process_count{0};
  double last_input_peak{0.0};
  double last_output_peak{0.0};
  double last_difference_peak{0.0};
  bool   first_process_done{false};

  // Thread-safe parameter change queue (no dynamic allocation)
  std::array<PendingParam, kMaxPending> pending_buf{};
  int                                   pending_count{0};
  std::mutex                            pending_mutex;  // protects pending_buf/count

  SimpleParamChanges   param_changes_obj;  // reused per process call
  ComponentHandlerImpl component_handler;  // installed on IEditController

  // ── Setup / shutdown ───────────────────────────────────────────────────────

  bool setup(double sample_rate) {
    const double sr = sample_rate > 0.0 ? sample_rate : 44100.0;

    // Wire up I/O buffer descriptors
    input_bus.numChannels       = 2;
    input_bus.channelBuffers32  = input_channels;
    output_bus.numChannels      = 2;
    output_bus.channelBuffers32 = output_channels;

    // ProcessData is reused every call — initialise once here
    process_data.processMode        = Steinberg::Vst::kRealtime;
    process_data.symbolicSampleSize = Steinberg::Vst::kSample32;
    process_data.numSamples         = 1;
    process_data.numInputs          = 1;
    process_data.numOutputs         = 1;
    process_data.inputs             = &input_bus;
    process_data.outputs            = &output_bus;
    process_data.inputParameterChanges  = nullptr;
    process_data.outputParameterChanges = nullptr;

    process_context.sampleRate         = sr;
    process_context.tempo              = 120.0;
    process_context.timeSigNumerator   = 4;
    process_context.timeSigDenominator = 4;
    process_context.state =
        Steinberg::Vst::ProcessContext::kTempoValid   |
        Steinberg::Vst::ProcessContext::kTimeSigValid |
        Steinberg::Vst::ProcessContext::kPlaying;
    process_data.processContext = &process_context;

    // Activate stereo buses
    const auto in_res  = component->activateBus(
        Steinberg::Vst::kAudio, Steinberg::Vst::kInput,  0, true);
    const auto out_res = component->activateBus(
        Steinberg::Vst::kAudio, Steinberg::Vst::kOutput, 0, true);
    if (in_res  != Steinberg::kResultOk)
      std::fprintf(stderr, "[DAUx VST3] activate input bus FAILED (result=%d)\n",  (int)in_res);
    if (out_res != Steinberg::kResultOk)
      std::fprintf(stderr, "[DAUx VST3] activate output bus FAILED (result=%d)\n", (int)out_res);

    processor->setBusArrangements(&input_arrangement, 1, &output_arrangement, 1);
    std::fprintf(stderr, "[DAUx VST3] buses activated: stereo in/out\n");

    // setupProcessing
    Steinberg::Vst::ProcessSetup ps{};
    ps.processMode        = Steinberg::Vst::kRealtime;
    ps.symbolicSampleSize = Steinberg::Vst::kSample32;
    ps.maxSamplesPerBlock = 1;
    ps.sampleRate         = sr;
    const auto setup_res = processor->setupProcessing(ps);
    if (setup_res != Steinberg::kResultOk) {
      std::fprintf(stderr, "[DAUx VST3] setupProcessing FAILED (result=%d)\n", (int)setup_res);
      return false;
    }
    std::fprintf(stderr, "[DAUx VST3] setupProcessing OK (sr=%.0f, maxBlock=1)\n", sr);

    // setActive(true)
    const auto active_res = component->setActive(true);
    if (active_res != Steinberg::kResultOk) {
      std::fprintf(stderr, "[DAUx VST3] setActive(true) FAILED (result=%d)\n", (int)active_res);
      return false;
    }

    // setProcessing(true)
    const auto proc_res = processor->setProcessing(true);
    if (proc_res != Steinberg::kResultOk) {
      std::fprintf(stderr, "[DAUx VST3] setProcessing(true) FAILED (result=%d)\n", (int)proc_res);
      return false;
    }
    processing = true;

    // Register IComponentHandler so plugin GUI edits are captured
    if (controller) {
      component_handler.owner = this;
      const auto ch_res = controller->setComponentHandler(&component_handler);
      if (ch_res == Steinberg::kResultOk)
        std::fprintf(stderr, "[DAUx VST3] IComponentHandler registered\n");
      else
        std::fprintf(stderr,
                     "[DAUx VST3] setComponentHandler not accepted (result=%d) — "
                     "GUI edits may not reach processor\n", (int)ch_res);
    }

    return true;
  }

  void shutdown() {
    if (processor && processing) processor->setProcessing(false);
    processing = false;
    if (component_connection && controller_connection) {
      component_connection->disconnect(controller_connection);
      controller_connection->disconnect(component_connection);
    }
    component_connection = nullptr;
    controller_connection = nullptr;
    if (controller && !controller_is_component) {
      if (auto pb = Steinberg::FUnknownPtr<Steinberg::IPluginBase>(controller))
        pb->terminate();
    }
    if (component) {
      component->setActive(false);
      if (auto pb = Steinberg::FUnknownPtr<Steinberg::IPluginBase>(component))
        pb->terminate();
    }
  }

  // ── Thread-safe parameter enqueue (called from audio thread OR GUI thread) ──

  /// Add or update a parameter change in the pending queue.
  /// Deduplicates by paramId — later value wins within one block.
  void enqueue_param(Steinberg::Vst::ParamID id, Steinberg::Vst::ParamValue value) {
    std::lock_guard<std::mutex> lock(pending_mutex);
    for (int i = 0; i < pending_count; ++i) {
      if (pending_buf[i].id == id) {
        pending_buf[i].value = value;
        return;
      }
    }
    if (pending_count < kMaxPending)
      pending_buf[pending_count++] = {id, value};
  }
};

// ── ComponentHandlerImpl::performEdit (needs full SphereDauxVst3Processor) ───

Steinberg::tresult PLUGIN_API ComponentHandlerImpl::performEdit(
    Steinberg::Vst::ParamID id,
    Steinberg::Vst::ParamValue value) {
  if (owner) owner->enqueue_param(id, value);
  return Steinberg::kResultOk;
}

// ── C API ─────────────────────────────────────────────────────────────────────

extern "C" SphereDauxVst3Processor* sphere_daux_vst3_create(
    const char* plugin_path,
    const char* class_id,
    double      sample_rate) {
  if (!plugin_path || !*plugin_path) return nullptr;

  auto instance = std::make_unique<SphereDauxVst3Processor>();
  std::string error;
  instance->module = VST3::Hosting::Module::create(plugin_path, error);
  if (!instance->module) {
    std::fprintf(stderr, "[DAUx VST3] module load FAILED: %s\n", error.c_str());
    return nullptr;
  }
  std::fprintf(stderr, "[DAUx VST3] plugin loaded: %s\n", plugin_path);

  const auto factory = instance->module->getFactory();
  factory.setHostContext(&instance->host_context);

  const std::string requested = class_id ? class_id : "";
  VST3::Optional<VST3::UID> uid;
  if (!looks_like_zero_class_id(requested))
    uid = VST3::UID::fromString(requested);
  if (!uid) uid = first_audio_module_uid(factory);
  if (!uid) {
    std::fprintf(stderr, "[DAUx VST3] no Audio Module Class found in factory\n");
    return nullptr;
  }

  instance->component = factory.createInstance<Steinberg::Vst::IComponent>(*uid);
  if (!instance->component) {
    std::fprintf(stderr, "[DAUx VST3] create IComponent FAILED\n");
    return nullptr;
  }
  if (auto pb = Steinberg::FUnknownPtr<Steinberg::IPluginBase>(instance->component)) {
    if (pb->initialize(&instance->host_context) != Steinberg::kResultOk) {
      std::fprintf(stderr, "[DAUx VST3] component initialize FAILED\n");
      return nullptr;
    }
  } else {
    std::fprintf(stderr, "[DAUx VST3] component does not implement IPluginBase\n");
    return nullptr;
  }

  if (instance->component->queryInterface(
          Steinberg::Vst::IAudioProcessor::iid,
          reinterpret_cast<void**>(&instance->processor)) != Steinberg::kResultTrue ||
      !instance->processor) {
    std::fprintf(stderr, "[DAUx VST3] component does not implement IAudioProcessor\n");
    return nullptr;
  }

  // Obtain IEditController (either from the component itself or a separate class)
  Steinberg::Vst::IEditController* raw_ctrl = nullptr;
  if (instance->component->queryInterface(
          Steinberg::Vst::IEditController::iid,
          reinterpret_cast<void**>(&raw_ctrl)) == Steinberg::kResultTrue) {
    instance->controller = Steinberg::IPtr<Steinberg::Vst::IEditController>::adopt(raw_ctrl);
    instance->controller_is_component = true;
  } else {
    Steinberg::TUID ctrl_cid{};
    if (instance->component->getControllerClassId(ctrl_cid) == Steinberg::kResultTrue) {
      instance->controller =
          factory.createInstance<Steinberg::Vst::IEditController>(VST3::UID(ctrl_cid));
      if (instance->controller) {
        if (auto pb = Steinberg::FUnknownPtr<Steinberg::IPluginBase>(instance->controller)) {
          if (pb->initialize(&instance->host_context) != Steinberg::kResultOk) {
            std::fprintf(stderr, "[DAUx VST3] controller initialize FAILED\n");
            instance->controller = nullptr;
          }
        }
      }
    }
  }

  // Connect component ↔ controller
  if (instance->controller) {
    instance->component_connection =
        Steinberg::FUnknownPtr<Steinberg::Vst::IConnectionPoint>(instance->component);
    instance->controller_connection =
        Steinberg::FUnknownPtr<Steinberg::Vst::IConnectionPoint>(instance->controller);
    if (instance->component_connection && instance->controller_connection) {
      instance->component_connection->connect(instance->controller_connection);
      instance->controller_connection->connect(instance->component_connection);
      std::fprintf(stderr, "[DAUx VST3] component/controller connected\n");
    }
  }

  if (!instance->setup(sample_rate)) {
    instance->shutdown();
    return nullptr;
  }

  std::fprintf(stderr, "[DAUx VST3] processor ready: %s\n", plugin_path);
  return instance.release();
}

extern "C" void sphere_daux_vst3_destroy(SphereDauxVst3Processor* processor) {
  if (!processor) return;
  processor->shutdown();
  delete processor;
}

extern "C" int sphere_daux_vst3_process_stereo_sample(
    SphereDauxVst3Processor* processor,
    float in_l, float in_r,
    float* out_l, float* out_r) {
  if (!processor || !processor->processor || !out_l || !out_r) return 0;

  // Drain pending parameter changes into inputParameterChanges.
  // Lock scope is minimal — no allocation occurs here.
  {
    std::lock_guard<std::mutex> lock(processor->pending_mutex);
    if (processor->pending_count > 0) {
      processor->param_changes_obj.reset();
      for (int i = 0; i < processor->pending_count; ++i) {
        Steinberg::int32 idx = 0;
        auto* q = processor->param_changes_obj.addParameterData(
            processor->pending_buf[i].id, idx);
        if (q) {
          Steinberg::int32 dummy = 0;
          q->addPoint(0, processor->pending_buf[i].value, dummy);
        }
      }
      processor->pending_count = 0;
      processor->process_data.inputParameterChanges = &processor->param_changes_obj;
    } else {
      processor->process_data.inputParameterChanges = nullptr;
    }
  }

  // Fill input, clear output
  processor->input_l  = in_l;
  processor->input_r  = in_r;
  processor->output_l = 0.f;
  processor->output_r = 0.f;

  const auto result = processor->processor->process(processor->process_data);

  // First-process debug log (fires once, outside the hot path thereafter)
  if (!processor->first_process_done) {
    processor->first_process_done = true;
    std::fprintf(stderr, "[DAUx VST3] first process call: %s\n",
                 result == Steinberg::kResultOk ? "OK" : "FAILED");
  }

  if (result != Steinberg::kResultOk) return 0;

  processor->process_count += 1;
  processor->last_input_peak = std::max(
      std::abs(static_cast<double>(in_l)),
      std::abs(static_cast<double>(in_r)));
  processor->last_output_peak = std::max(
      std::abs(static_cast<double>(processor->output_l)),
      std::abs(static_cast<double>(processor->output_r)));
  processor->last_difference_peak = std::max(
      std::abs(static_cast<double>(processor->output_l - in_l)),
      std::abs(static_cast<double>(processor->output_r - in_r)));

  *out_l = processor->output_l;
  *out_r = processor->output_r;
  return 1;
}

/// Enqueue a normalized parameter change for delivery on the next process call.
/// Safe to call from any thread (audio thread or UI thread).
extern "C" void sphere_daux_vst3_set_param(
    SphereDauxVst3Processor* processor,
    unsigned int             param_id,
    double                   value) {
  if (!processor) return;
  processor->enqueue_param(
      static_cast<Steinberg::Vst::ParamID>(param_id),
      static_cast<Steinberg::Vst::ParamValue>(value));
}

extern "C" unsigned long long sphere_daux_vst3_process_count(
    SphereDauxVst3Processor* processor) {
  return processor ? processor->process_count : 0;
}

extern "C" double sphere_daux_vst3_last_input_peak(SphereDauxVst3Processor* processor) {
  return processor ? processor->last_input_peak : 0.0;
}

extern "C" double sphere_daux_vst3_last_output_peak(SphereDauxVst3Processor* processor) {
  return processor ? processor->last_output_peak : 0.0;
}

extern "C" double sphere_daux_vst3_last_difference_peak(SphereDauxVst3Processor* processor) {
  return processor ? processor->last_difference_peak : 0.0;
}
