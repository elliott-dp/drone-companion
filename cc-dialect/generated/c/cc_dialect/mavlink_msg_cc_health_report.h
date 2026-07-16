#pragma once
// MESSAGE CC_HEALTH_REPORT PACKING

#define MAVLINK_MSG_ID_CC_HEALTH_REPORT 54010


typedef struct __mavlink_cc_health_report_t {
 uint64_t companion_timestamp_us; /*< [us] CC monotonic time at report creation.*/
 uint32_t sequence; /*<  Report sequence, monotonic per cc_boot_id.*/
 uint32_t mission_id; /*<  Current mission identity.*/
 uint32_t companion_boot_id; /*<  companiond process boot identity.*/
 uint32_t health_flags; /*<  Active health concern bitmask.*/
 uint16_t detail_code; /*<  Dominant evidence code (joins to CC_AI_DIAGNOSTIC / ai_health.parquet).*/
 uint16_t link_rtt_ms; /*< [ms] Companion-measured link round-trip time.*/
 uint16_t telemetry_age_ms; /*< [ms] Age of newest FC telemetry at report time.*/
 uint16_t companion_loop_ms; /*< [ms] Health evaluation loop duration (self-telemetry).*/
 uint16_t dropped_rx_count; /*<  Frames dropped by companion since boot (self-telemetry).*/
 uint8_t severity; /*<  Worst-case merged severity.*/
 uint8_t recommended_action; /*<  Advisory recommended action.*/
 uint8_t confidence_percent; /*< [%] Confidence in the conclusion [0..100].*/
 uint8_t schema_version; /*<  Payload schema version.*/
} mavlink_cc_health_report_t;

#define MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN 38
#define MAVLINK_MSG_ID_CC_HEALTH_REPORT_MIN_LEN 38
#define MAVLINK_MSG_ID_54010_LEN 38
#define MAVLINK_MSG_ID_54010_MIN_LEN 38

#define MAVLINK_MSG_ID_CC_HEALTH_REPORT_CRC 76
#define MAVLINK_MSG_ID_54010_CRC 76



#if MAVLINK_COMMAND_24BIT
#define MAVLINK_MESSAGE_INFO_CC_HEALTH_REPORT { \
    54010, \
    "CC_HEALTH_REPORT", \
    14, \
    {  { "companion_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_health_report_t, companion_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_health_report_t, sequence) }, \
         { "mission_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 12, offsetof(mavlink_cc_health_report_t, mission_id) }, \
         { "companion_boot_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 16, offsetof(mavlink_cc_health_report_t, companion_boot_id) }, \
         { "health_flags", NULL, MAVLINK_TYPE_UINT32_T, 0, 20, offsetof(mavlink_cc_health_report_t, health_flags) }, \
         { "detail_code", NULL, MAVLINK_TYPE_UINT16_T, 0, 24, offsetof(mavlink_cc_health_report_t, detail_code) }, \
         { "link_rtt_ms", NULL, MAVLINK_TYPE_UINT16_T, 0, 26, offsetof(mavlink_cc_health_report_t, link_rtt_ms) }, \
         { "telemetry_age_ms", NULL, MAVLINK_TYPE_UINT16_T, 0, 28, offsetof(mavlink_cc_health_report_t, telemetry_age_ms) }, \
         { "companion_loop_ms", NULL, MAVLINK_TYPE_UINT16_T, 0, 30, offsetof(mavlink_cc_health_report_t, companion_loop_ms) }, \
         { "dropped_rx_count", NULL, MAVLINK_TYPE_UINT16_T, 0, 32, offsetof(mavlink_cc_health_report_t, dropped_rx_count) }, \
         { "severity", NULL, MAVLINK_TYPE_UINT8_T, 0, 34, offsetof(mavlink_cc_health_report_t, severity) }, \
         { "recommended_action", NULL, MAVLINK_TYPE_UINT8_T, 0, 35, offsetof(mavlink_cc_health_report_t, recommended_action) }, \
         { "confidence_percent", NULL, MAVLINK_TYPE_UINT8_T, 0, 36, offsetof(mavlink_cc_health_report_t, confidence_percent) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 37, offsetof(mavlink_cc_health_report_t, schema_version) }, \
         } \
}
#else
#define MAVLINK_MESSAGE_INFO_CC_HEALTH_REPORT { \
    "CC_HEALTH_REPORT", \
    14, \
    {  { "companion_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_health_report_t, companion_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_health_report_t, sequence) }, \
         { "mission_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 12, offsetof(mavlink_cc_health_report_t, mission_id) }, \
         { "companion_boot_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 16, offsetof(mavlink_cc_health_report_t, companion_boot_id) }, \
         { "health_flags", NULL, MAVLINK_TYPE_UINT32_T, 0, 20, offsetof(mavlink_cc_health_report_t, health_flags) }, \
         { "detail_code", NULL, MAVLINK_TYPE_UINT16_T, 0, 24, offsetof(mavlink_cc_health_report_t, detail_code) }, \
         { "link_rtt_ms", NULL, MAVLINK_TYPE_UINT16_T, 0, 26, offsetof(mavlink_cc_health_report_t, link_rtt_ms) }, \
         { "telemetry_age_ms", NULL, MAVLINK_TYPE_UINT16_T, 0, 28, offsetof(mavlink_cc_health_report_t, telemetry_age_ms) }, \
         { "companion_loop_ms", NULL, MAVLINK_TYPE_UINT16_T, 0, 30, offsetof(mavlink_cc_health_report_t, companion_loop_ms) }, \
         { "dropped_rx_count", NULL, MAVLINK_TYPE_UINT16_T, 0, 32, offsetof(mavlink_cc_health_report_t, dropped_rx_count) }, \
         { "severity", NULL, MAVLINK_TYPE_UINT8_T, 0, 34, offsetof(mavlink_cc_health_report_t, severity) }, \
         { "recommended_action", NULL, MAVLINK_TYPE_UINT8_T, 0, 35, offsetof(mavlink_cc_health_report_t, recommended_action) }, \
         { "confidence_percent", NULL, MAVLINK_TYPE_UINT8_T, 0, 36, offsetof(mavlink_cc_health_report_t, confidence_percent) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 37, offsetof(mavlink_cc_health_report_t, schema_version) }, \
         } \
}
#endif

/**
 * @brief Pack a cc_health_report message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 *
 * @param companion_timestamp_us [us] CC monotonic time at report creation.
 * @param sequence  Report sequence, monotonic per cc_boot_id.
 * @param mission_id  Current mission identity.
 * @param companion_boot_id  companiond process boot identity.
 * @param health_flags  Active health concern bitmask.
 * @param detail_code  Dominant evidence code (joins to CC_AI_DIAGNOSTIC / ai_health.parquet).
 * @param link_rtt_ms [ms] Companion-measured link round-trip time.
 * @param telemetry_age_ms [ms] Age of newest FC telemetry at report time.
 * @param companion_loop_ms [ms] Health evaluation loop duration (self-telemetry).
 * @param dropped_rx_count  Frames dropped by companion since boot (self-telemetry).
 * @param severity  Worst-case merged severity.
 * @param recommended_action  Advisory recommended action.
 * @param confidence_percent [%] Confidence in the conclusion [0..100].
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_health_report_pack(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg,
                               uint64_t companion_timestamp_us, uint32_t sequence, uint32_t mission_id, uint32_t companion_boot_id, uint32_t health_flags, uint16_t detail_code, uint16_t link_rtt_ms, uint16_t telemetry_age_ms, uint16_t companion_loop_ms, uint16_t dropped_rx_count, uint8_t severity, uint8_t recommended_action, uint8_t confidence_percent, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN];
    _mav_put_uint64_t(buf, 0, companion_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, mission_id);
    _mav_put_uint32_t(buf, 16, companion_boot_id);
    _mav_put_uint32_t(buf, 20, health_flags);
    _mav_put_uint16_t(buf, 24, detail_code);
    _mav_put_uint16_t(buf, 26, link_rtt_ms);
    _mav_put_uint16_t(buf, 28, telemetry_age_ms);
    _mav_put_uint16_t(buf, 30, companion_loop_ms);
    _mav_put_uint16_t(buf, 32, dropped_rx_count);
    _mav_put_uint8_t(buf, 34, severity);
    _mav_put_uint8_t(buf, 35, recommended_action);
    _mav_put_uint8_t(buf, 36, confidence_percent);
    _mav_put_uint8_t(buf, 37, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN);
#else
    mavlink_cc_health_report_t packet;
    packet.companion_timestamp_us = companion_timestamp_us;
    packet.sequence = sequence;
    packet.mission_id = mission_id;
    packet.companion_boot_id = companion_boot_id;
    packet.health_flags = health_flags;
    packet.detail_code = detail_code;
    packet.link_rtt_ms = link_rtt_ms;
    packet.telemetry_age_ms = telemetry_age_ms;
    packet.companion_loop_ms = companion_loop_ms;
    packet.dropped_rx_count = dropped_rx_count;
    packet.severity = severity;
    packet.recommended_action = recommended_action;
    packet.confidence_percent = confidence_percent;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_HEALTH_REPORT;
    return mavlink_finalize_message(msg, system_id, component_id, MAVLINK_MSG_ID_CC_HEALTH_REPORT_MIN_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_CRC);
}

/**
 * @brief Pack a cc_health_report message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 *
 * @param companion_timestamp_us [us] CC monotonic time at report creation.
 * @param sequence  Report sequence, monotonic per cc_boot_id.
 * @param mission_id  Current mission identity.
 * @param companion_boot_id  companiond process boot identity.
 * @param health_flags  Active health concern bitmask.
 * @param detail_code  Dominant evidence code (joins to CC_AI_DIAGNOSTIC / ai_health.parquet).
 * @param link_rtt_ms [ms] Companion-measured link round-trip time.
 * @param telemetry_age_ms [ms] Age of newest FC telemetry at report time.
 * @param companion_loop_ms [ms] Health evaluation loop duration (self-telemetry).
 * @param dropped_rx_count  Frames dropped by companion since boot (self-telemetry).
 * @param severity  Worst-case merged severity.
 * @param recommended_action  Advisory recommended action.
 * @param confidence_percent [%] Confidence in the conclusion [0..100].
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_health_report_pack_status(uint8_t system_id, uint8_t component_id, mavlink_status_t *_status, mavlink_message_t* msg,
                               uint64_t companion_timestamp_us, uint32_t sequence, uint32_t mission_id, uint32_t companion_boot_id, uint32_t health_flags, uint16_t detail_code, uint16_t link_rtt_ms, uint16_t telemetry_age_ms, uint16_t companion_loop_ms, uint16_t dropped_rx_count, uint8_t severity, uint8_t recommended_action, uint8_t confidence_percent, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN];
    _mav_put_uint64_t(buf, 0, companion_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, mission_id);
    _mav_put_uint32_t(buf, 16, companion_boot_id);
    _mav_put_uint32_t(buf, 20, health_flags);
    _mav_put_uint16_t(buf, 24, detail_code);
    _mav_put_uint16_t(buf, 26, link_rtt_ms);
    _mav_put_uint16_t(buf, 28, telemetry_age_ms);
    _mav_put_uint16_t(buf, 30, companion_loop_ms);
    _mav_put_uint16_t(buf, 32, dropped_rx_count);
    _mav_put_uint8_t(buf, 34, severity);
    _mav_put_uint8_t(buf, 35, recommended_action);
    _mav_put_uint8_t(buf, 36, confidence_percent);
    _mav_put_uint8_t(buf, 37, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN);
#else
    mavlink_cc_health_report_t packet;
    packet.companion_timestamp_us = companion_timestamp_us;
    packet.sequence = sequence;
    packet.mission_id = mission_id;
    packet.companion_boot_id = companion_boot_id;
    packet.health_flags = health_flags;
    packet.detail_code = detail_code;
    packet.link_rtt_ms = link_rtt_ms;
    packet.telemetry_age_ms = telemetry_age_ms;
    packet.companion_loop_ms = companion_loop_ms;
    packet.dropped_rx_count = dropped_rx_count;
    packet.severity = severity;
    packet.recommended_action = recommended_action;
    packet.confidence_percent = confidence_percent;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_HEALTH_REPORT;
#if MAVLINK_CRC_EXTRA
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_HEALTH_REPORT_MIN_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_CRC);
#else
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_HEALTH_REPORT_MIN_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN);
#endif
}

/**
 * @brief Pack a cc_health_report message on a channel
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param companion_timestamp_us [us] CC monotonic time at report creation.
 * @param sequence  Report sequence, monotonic per cc_boot_id.
 * @param mission_id  Current mission identity.
 * @param companion_boot_id  companiond process boot identity.
 * @param health_flags  Active health concern bitmask.
 * @param detail_code  Dominant evidence code (joins to CC_AI_DIAGNOSTIC / ai_health.parquet).
 * @param link_rtt_ms [ms] Companion-measured link round-trip time.
 * @param telemetry_age_ms [ms] Age of newest FC telemetry at report time.
 * @param companion_loop_ms [ms] Health evaluation loop duration (self-telemetry).
 * @param dropped_rx_count  Frames dropped by companion since boot (self-telemetry).
 * @param severity  Worst-case merged severity.
 * @param recommended_action  Advisory recommended action.
 * @param confidence_percent [%] Confidence in the conclusion [0..100].
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_health_report_pack_chan(uint8_t system_id, uint8_t component_id, uint8_t chan,
                               mavlink_message_t* msg,
                                   uint64_t companion_timestamp_us,uint32_t sequence,uint32_t mission_id,uint32_t companion_boot_id,uint32_t health_flags,uint16_t detail_code,uint16_t link_rtt_ms,uint16_t telemetry_age_ms,uint16_t companion_loop_ms,uint16_t dropped_rx_count,uint8_t severity,uint8_t recommended_action,uint8_t confidence_percent,uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN];
    _mav_put_uint64_t(buf, 0, companion_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, mission_id);
    _mav_put_uint32_t(buf, 16, companion_boot_id);
    _mav_put_uint32_t(buf, 20, health_flags);
    _mav_put_uint16_t(buf, 24, detail_code);
    _mav_put_uint16_t(buf, 26, link_rtt_ms);
    _mav_put_uint16_t(buf, 28, telemetry_age_ms);
    _mav_put_uint16_t(buf, 30, companion_loop_ms);
    _mav_put_uint16_t(buf, 32, dropped_rx_count);
    _mav_put_uint8_t(buf, 34, severity);
    _mav_put_uint8_t(buf, 35, recommended_action);
    _mav_put_uint8_t(buf, 36, confidence_percent);
    _mav_put_uint8_t(buf, 37, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN);
#else
    mavlink_cc_health_report_t packet;
    packet.companion_timestamp_us = companion_timestamp_us;
    packet.sequence = sequence;
    packet.mission_id = mission_id;
    packet.companion_boot_id = companion_boot_id;
    packet.health_flags = health_flags;
    packet.detail_code = detail_code;
    packet.link_rtt_ms = link_rtt_ms;
    packet.telemetry_age_ms = telemetry_age_ms;
    packet.companion_loop_ms = companion_loop_ms;
    packet.dropped_rx_count = dropped_rx_count;
    packet.severity = severity;
    packet.recommended_action = recommended_action;
    packet.confidence_percent = confidence_percent;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_HEALTH_REPORT;
    return mavlink_finalize_message_chan(msg, system_id, component_id, chan, MAVLINK_MSG_ID_CC_HEALTH_REPORT_MIN_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_CRC);
}

/**
 * @brief Encode a cc_health_report struct
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 * @param cc_health_report C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_health_report_encode(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg, const mavlink_cc_health_report_t* cc_health_report)
{
    return mavlink_msg_cc_health_report_pack(system_id, component_id, msg, cc_health_report->companion_timestamp_us, cc_health_report->sequence, cc_health_report->mission_id, cc_health_report->companion_boot_id, cc_health_report->health_flags, cc_health_report->detail_code, cc_health_report->link_rtt_ms, cc_health_report->telemetry_age_ms, cc_health_report->companion_loop_ms, cc_health_report->dropped_rx_count, cc_health_report->severity, cc_health_report->recommended_action, cc_health_report->confidence_percent, cc_health_report->schema_version);
}

/**
 * @brief Encode a cc_health_report struct on a channel
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param cc_health_report C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_health_report_encode_chan(uint8_t system_id, uint8_t component_id, uint8_t chan, mavlink_message_t* msg, const mavlink_cc_health_report_t* cc_health_report)
{
    return mavlink_msg_cc_health_report_pack_chan(system_id, component_id, chan, msg, cc_health_report->companion_timestamp_us, cc_health_report->sequence, cc_health_report->mission_id, cc_health_report->companion_boot_id, cc_health_report->health_flags, cc_health_report->detail_code, cc_health_report->link_rtt_ms, cc_health_report->telemetry_age_ms, cc_health_report->companion_loop_ms, cc_health_report->dropped_rx_count, cc_health_report->severity, cc_health_report->recommended_action, cc_health_report->confidence_percent, cc_health_report->schema_version);
}

/**
 * @brief Encode a cc_health_report struct with provided status structure
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 * @param cc_health_report C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_health_report_encode_status(uint8_t system_id, uint8_t component_id, mavlink_status_t* _status, mavlink_message_t* msg, const mavlink_cc_health_report_t* cc_health_report)
{
    return mavlink_msg_cc_health_report_pack_status(system_id, component_id, _status, msg,  cc_health_report->companion_timestamp_us, cc_health_report->sequence, cc_health_report->mission_id, cc_health_report->companion_boot_id, cc_health_report->health_flags, cc_health_report->detail_code, cc_health_report->link_rtt_ms, cc_health_report->telemetry_age_ms, cc_health_report->companion_loop_ms, cc_health_report->dropped_rx_count, cc_health_report->severity, cc_health_report->recommended_action, cc_health_report->confidence_percent, cc_health_report->schema_version);
}

/**
 * @brief Send a cc_health_report message
 * @param chan MAVLink channel to send the message
 *
 * @param companion_timestamp_us [us] CC monotonic time at report creation.
 * @param sequence  Report sequence, monotonic per cc_boot_id.
 * @param mission_id  Current mission identity.
 * @param companion_boot_id  companiond process boot identity.
 * @param health_flags  Active health concern bitmask.
 * @param detail_code  Dominant evidence code (joins to CC_AI_DIAGNOSTIC / ai_health.parquet).
 * @param link_rtt_ms [ms] Companion-measured link round-trip time.
 * @param telemetry_age_ms [ms] Age of newest FC telemetry at report time.
 * @param companion_loop_ms [ms] Health evaluation loop duration (self-telemetry).
 * @param dropped_rx_count  Frames dropped by companion since boot (self-telemetry).
 * @param severity  Worst-case merged severity.
 * @param recommended_action  Advisory recommended action.
 * @param confidence_percent [%] Confidence in the conclusion [0..100].
 * @param schema_version  Payload schema version.
 */
#ifdef MAVLINK_USE_CONVENIENCE_FUNCTIONS

static inline void mavlink_msg_cc_health_report_send(mavlink_channel_t chan, uint64_t companion_timestamp_us, uint32_t sequence, uint32_t mission_id, uint32_t companion_boot_id, uint32_t health_flags, uint16_t detail_code, uint16_t link_rtt_ms, uint16_t telemetry_age_ms, uint16_t companion_loop_ms, uint16_t dropped_rx_count, uint8_t severity, uint8_t recommended_action, uint8_t confidence_percent, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN];
    _mav_put_uint64_t(buf, 0, companion_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, mission_id);
    _mav_put_uint32_t(buf, 16, companion_boot_id);
    _mav_put_uint32_t(buf, 20, health_flags);
    _mav_put_uint16_t(buf, 24, detail_code);
    _mav_put_uint16_t(buf, 26, link_rtt_ms);
    _mav_put_uint16_t(buf, 28, telemetry_age_ms);
    _mav_put_uint16_t(buf, 30, companion_loop_ms);
    _mav_put_uint16_t(buf, 32, dropped_rx_count);
    _mav_put_uint8_t(buf, 34, severity);
    _mav_put_uint8_t(buf, 35, recommended_action);
    _mav_put_uint8_t(buf, 36, confidence_percent);
    _mav_put_uint8_t(buf, 37, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_HEALTH_REPORT, buf, MAVLINK_MSG_ID_CC_HEALTH_REPORT_MIN_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_CRC);
#else
    mavlink_cc_health_report_t packet;
    packet.companion_timestamp_us = companion_timestamp_us;
    packet.sequence = sequence;
    packet.mission_id = mission_id;
    packet.companion_boot_id = companion_boot_id;
    packet.health_flags = health_flags;
    packet.detail_code = detail_code;
    packet.link_rtt_ms = link_rtt_ms;
    packet.telemetry_age_ms = telemetry_age_ms;
    packet.companion_loop_ms = companion_loop_ms;
    packet.dropped_rx_count = dropped_rx_count;
    packet.severity = severity;
    packet.recommended_action = recommended_action;
    packet.confidence_percent = confidence_percent;
    packet.schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_HEALTH_REPORT, (const char *)&packet, MAVLINK_MSG_ID_CC_HEALTH_REPORT_MIN_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_CRC);
#endif
}

/**
 * @brief Send a cc_health_report message
 * @param chan MAVLink channel to send the message
 * @param struct The MAVLink struct to serialize
 */
static inline void mavlink_msg_cc_health_report_send_struct(mavlink_channel_t chan, const mavlink_cc_health_report_t* cc_health_report)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    mavlink_msg_cc_health_report_send(chan, cc_health_report->companion_timestamp_us, cc_health_report->sequence, cc_health_report->mission_id, cc_health_report->companion_boot_id, cc_health_report->health_flags, cc_health_report->detail_code, cc_health_report->link_rtt_ms, cc_health_report->telemetry_age_ms, cc_health_report->companion_loop_ms, cc_health_report->dropped_rx_count, cc_health_report->severity, cc_health_report->recommended_action, cc_health_report->confidence_percent, cc_health_report->schema_version);
#else
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_HEALTH_REPORT, (const char *)cc_health_report, MAVLINK_MSG_ID_CC_HEALTH_REPORT_MIN_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_CRC);
#endif
}

#if MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN <= MAVLINK_MAX_PAYLOAD_LEN
/*
  This variant of _send() can be used to save stack space by reusing
  memory from the receive buffer.  The caller provides a
  mavlink_message_t which is the size of a full mavlink message. This
  is usually the receive buffer for the channel, and allows a reply to an
  incoming message with minimum stack space usage.
 */
static inline void mavlink_msg_cc_health_report_send_buf(mavlink_message_t *msgbuf, mavlink_channel_t chan,  uint64_t companion_timestamp_us, uint32_t sequence, uint32_t mission_id, uint32_t companion_boot_id, uint32_t health_flags, uint16_t detail_code, uint16_t link_rtt_ms, uint16_t telemetry_age_ms, uint16_t companion_loop_ms, uint16_t dropped_rx_count, uint8_t severity, uint8_t recommended_action, uint8_t confidence_percent, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char *buf = (char *)msgbuf;
    _mav_put_uint64_t(buf, 0, companion_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, mission_id);
    _mav_put_uint32_t(buf, 16, companion_boot_id);
    _mav_put_uint32_t(buf, 20, health_flags);
    _mav_put_uint16_t(buf, 24, detail_code);
    _mav_put_uint16_t(buf, 26, link_rtt_ms);
    _mav_put_uint16_t(buf, 28, telemetry_age_ms);
    _mav_put_uint16_t(buf, 30, companion_loop_ms);
    _mav_put_uint16_t(buf, 32, dropped_rx_count);
    _mav_put_uint8_t(buf, 34, severity);
    _mav_put_uint8_t(buf, 35, recommended_action);
    _mav_put_uint8_t(buf, 36, confidence_percent);
    _mav_put_uint8_t(buf, 37, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_HEALTH_REPORT, buf, MAVLINK_MSG_ID_CC_HEALTH_REPORT_MIN_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_CRC);
#else
    mavlink_cc_health_report_t *packet = (mavlink_cc_health_report_t *)msgbuf;
    packet->companion_timestamp_us = companion_timestamp_us;
    packet->sequence = sequence;
    packet->mission_id = mission_id;
    packet->companion_boot_id = companion_boot_id;
    packet->health_flags = health_flags;
    packet->detail_code = detail_code;
    packet->link_rtt_ms = link_rtt_ms;
    packet->telemetry_age_ms = telemetry_age_ms;
    packet->companion_loop_ms = companion_loop_ms;
    packet->dropped_rx_count = dropped_rx_count;
    packet->severity = severity;
    packet->recommended_action = recommended_action;
    packet->confidence_percent = confidence_percent;
    packet->schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_HEALTH_REPORT, (const char *)packet, MAVLINK_MSG_ID_CC_HEALTH_REPORT_MIN_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN, MAVLINK_MSG_ID_CC_HEALTH_REPORT_CRC);
#endif
}
#endif

#endif

// MESSAGE CC_HEALTH_REPORT UNPACKING


/**
 * @brief Get field companion_timestamp_us from cc_health_report message
 *
 * @return [us] CC monotonic time at report creation.
 */
static inline uint64_t mavlink_msg_cc_health_report_get_companion_timestamp_us(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint64_t(msg,  0);
}

/**
 * @brief Get field sequence from cc_health_report message
 *
 * @return  Report sequence, monotonic per cc_boot_id.
 */
static inline uint32_t mavlink_msg_cc_health_report_get_sequence(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  8);
}

/**
 * @brief Get field mission_id from cc_health_report message
 *
 * @return  Current mission identity.
 */
static inline uint32_t mavlink_msg_cc_health_report_get_mission_id(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  12);
}

/**
 * @brief Get field companion_boot_id from cc_health_report message
 *
 * @return  companiond process boot identity.
 */
static inline uint32_t mavlink_msg_cc_health_report_get_companion_boot_id(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  16);
}

/**
 * @brief Get field health_flags from cc_health_report message
 *
 * @return  Active health concern bitmask.
 */
static inline uint32_t mavlink_msg_cc_health_report_get_health_flags(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  20);
}

/**
 * @brief Get field detail_code from cc_health_report message
 *
 * @return  Dominant evidence code (joins to CC_AI_DIAGNOSTIC / ai_health.parquet).
 */
static inline uint16_t mavlink_msg_cc_health_report_get_detail_code(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint16_t(msg,  24);
}

/**
 * @brief Get field link_rtt_ms from cc_health_report message
 *
 * @return [ms] Companion-measured link round-trip time.
 */
static inline uint16_t mavlink_msg_cc_health_report_get_link_rtt_ms(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint16_t(msg,  26);
}

/**
 * @brief Get field telemetry_age_ms from cc_health_report message
 *
 * @return [ms] Age of newest FC telemetry at report time.
 */
static inline uint16_t mavlink_msg_cc_health_report_get_telemetry_age_ms(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint16_t(msg,  28);
}

/**
 * @brief Get field companion_loop_ms from cc_health_report message
 *
 * @return [ms] Health evaluation loop duration (self-telemetry).
 */
static inline uint16_t mavlink_msg_cc_health_report_get_companion_loop_ms(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint16_t(msg,  30);
}

/**
 * @brief Get field dropped_rx_count from cc_health_report message
 *
 * @return  Frames dropped by companion since boot (self-telemetry).
 */
static inline uint16_t mavlink_msg_cc_health_report_get_dropped_rx_count(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint16_t(msg,  32);
}

/**
 * @brief Get field severity from cc_health_report message
 *
 * @return  Worst-case merged severity.
 */
static inline uint8_t mavlink_msg_cc_health_report_get_severity(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  34);
}

/**
 * @brief Get field recommended_action from cc_health_report message
 *
 * @return  Advisory recommended action.
 */
static inline uint8_t mavlink_msg_cc_health_report_get_recommended_action(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  35);
}

/**
 * @brief Get field confidence_percent from cc_health_report message
 *
 * @return [%] Confidence in the conclusion [0..100].
 */
static inline uint8_t mavlink_msg_cc_health_report_get_confidence_percent(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  36);
}

/**
 * @brief Get field schema_version from cc_health_report message
 *
 * @return  Payload schema version.
 */
static inline uint8_t mavlink_msg_cc_health_report_get_schema_version(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  37);
}

/**
 * @brief Decode a cc_health_report message into a struct
 *
 * @param msg The message to decode
 * @param cc_health_report C-struct to decode the message contents into
 */
static inline void mavlink_msg_cc_health_report_decode(const mavlink_message_t* msg, mavlink_cc_health_report_t* cc_health_report)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    cc_health_report->companion_timestamp_us = mavlink_msg_cc_health_report_get_companion_timestamp_us(msg);
    cc_health_report->sequence = mavlink_msg_cc_health_report_get_sequence(msg);
    cc_health_report->mission_id = mavlink_msg_cc_health_report_get_mission_id(msg);
    cc_health_report->companion_boot_id = mavlink_msg_cc_health_report_get_companion_boot_id(msg);
    cc_health_report->health_flags = mavlink_msg_cc_health_report_get_health_flags(msg);
    cc_health_report->detail_code = mavlink_msg_cc_health_report_get_detail_code(msg);
    cc_health_report->link_rtt_ms = mavlink_msg_cc_health_report_get_link_rtt_ms(msg);
    cc_health_report->telemetry_age_ms = mavlink_msg_cc_health_report_get_telemetry_age_ms(msg);
    cc_health_report->companion_loop_ms = mavlink_msg_cc_health_report_get_companion_loop_ms(msg);
    cc_health_report->dropped_rx_count = mavlink_msg_cc_health_report_get_dropped_rx_count(msg);
    cc_health_report->severity = mavlink_msg_cc_health_report_get_severity(msg);
    cc_health_report->recommended_action = mavlink_msg_cc_health_report_get_recommended_action(msg);
    cc_health_report->confidence_percent = mavlink_msg_cc_health_report_get_confidence_percent(msg);
    cc_health_report->schema_version = mavlink_msg_cc_health_report_get_schema_version(msg);
#else
        uint8_t len = msg->len < MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN? msg->len : MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN;
        memset(cc_health_report, 0, MAVLINK_MSG_ID_CC_HEALTH_REPORT_LEN);
    memcpy(cc_health_report, _MAV_PAYLOAD(msg), len);
#endif
}
