# Remaining 100% Checklist

## High-level XTypes
- [ ] Rich `TypeInfo` API
- [ ] Rich `TypeObject` API
- [ ] Type metadata summaries/helpers
- [ ] Real tests for metadata relationships

## Dynamic types
- [x] Structure / enum / union / bitmask / alias builders
- [x] Member properties (`key`, `optional`, `external`, `must_understand`, `hash_id`)
- [x] Member insertion positions
- [ ] Default-value story closed
- [ ] Broader invalid-matrix coverage completed
- [ ] Build-sensitive feature behavior catalogued

## Dynamic values
- [ ] Dynamic value abstraction
- [ ] Primitive field get/set
- [ ] Union discriminator/value handling
- [ ] Sequence/array/string dynamic field access
- [ ] Read/write dynamic values through DDS

## Built-in topics / discovery
- [ ] Full `DCPSParticipant` wrapper
- [ ] Full `DCPSTopic` wrapper
- [ ] Full `DCPSPublication` wrapper
- [ ] Full `DCPSSubscription` wrapper
- [ ] Built-in reader flows stabilized
- [ ] Multi-process discovery coverage

## Advanced feature groups
- [ ] Content filtered topics
- [ ] Topic filters/query refinements
- [ ] Shared-memory/build-sensitive APIs finalized
- [ ] Remaining advanced public API groups closed

## Release readiness
- [ ] Examples for major feature groups
- [ ] Build-sensitive behavior documented
- [ ] Stress / long-form test notes
- [ ] All green on fmt/check/clippy/tests
- [ ] Honest 100% claim criteria satisfied
