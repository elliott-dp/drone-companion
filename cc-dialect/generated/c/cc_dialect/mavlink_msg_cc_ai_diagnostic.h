#pragma once
// MESSAGE CC_AI_DIAGNOSTIC PACKING

#define MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC 54011


typedef struct __mavlink_cc_ai_diagnostic_t {
 uint64_t companion_timestamp_us; /*< [us] CC monotonic time at finding creation.*/
 uint32_t sequence; /*<  Diagnostic sequence, monotonic per cc_boot_id.*/
 float value; /*<  Measured value that triggered the finding.*/
 float limit; /*<  Threshold/limit the value was compared against.*/
 uint16_t detail_code; /*<  Evidence code (same namespace as CC_HEALTH_REPORT.detail_code).*/
 uint8_t subsystem; /*<  Subsystem of the finding.*/
 uint8_t severity; /*<  Severity of this individual finding.*/
 uint8_t confidence_percent; /*< [%] Confidence [0..100].*/
 uint8_t schema_version; /*<  Payload schema version.*/
} mavlink_cc_ai_diagnostic_t;

#define MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN 26
#define MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_MIN_LEN 26
#define MAVLINK_MSG_ID_54011_LEN 26
#define MAVLINK_MSG_ID_54011_MIN_LEN 26

#define MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_CRC 83
#define MAVLINK_MSG_ID_54011_CRC 83



#if MAVLINK_COMMAND_24BIT
#define MAVLINK_MESSAGE_INFO_CC_AI_DIAGNOSTIC { \
    54011, \
    "CC_AI_DIAGNOSTIC", \
    9, \
    {  { "companion_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_ai_diagnostic_t, companion_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_ai_diagnostic_t, sequence) }, \
         { "value", NULL, MAVLINK_TYPE_FLOAT, 0, 12, offsetof(mavlink_cc_ai_diagnostic_t, value) }, \
         { "limit", NULL, MAVLINK_TYPE_FLOAT, 0, 16, offsetof(mavlink_cc_ai_diagnostic_t, limit) }, \
         { "detail_code", NULL, MAVLINK_TYPE_UINT16_T, 0, 20, offsetof(mavlink_cc_ai_diagnostic_t, detail_code) }, \
         { "subsystem", NULL, MAVLINK_TYPE_UINT8_T, 0, 22, offsetof(mavlink_cc_ai_diagnostic_t, subsystem) }, \
         { "severity", NULL, MAVLINK_TYPE_UINT8_T, 0, 23, offsetof(mavlink_cc_ai_diagnostic_t, severity) }, \
         { "confidence_percent", NULL, MAVLINK_TYPE_UINT8_T, 0, 24, offsetof(mavlink_cc_ai_diagnostic_t, confidence_percent) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 25, offsetof(mavlink_cc_ai_diagnostic_t, schema_version) }, \
         } \
}
#else
#define MAVLINK_MESSAGE_INFO_CC_AI_DIAGNOSTIC { \
    "CC_AI_DIAGNOSTIC", \
    9, \
    {  { "companion_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_ai_diagnostic_t, companion_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_ai_diagnostic_t, sequence) }, \
         { "value", NULL, MAVLINK_TYPE_FLOAT, 0, 12, offsetof(mavlink_cc_ai_diagnostic_t, value) }, \
         { "limit", NULL, MAVLINK_TYPE_FLOAT, 0, 16, offsetof(mavlink_cc_ai_diagnostic_t, limit) }, \
         { "detail_code", NULL, MAVLINK_TYPE_UINT16_T, 0, 20, offsetof(mavlink_cc_ai_diagnostic_t, detail_code) }, \
         { "subsystem", NULL, MAVLINK_TYPE_UINT8_T, 0, 22, offsetof(mavlink_cc_ai_diagnostic_t, subsystem) }, \
         { "severity", NULL, MAVLINK_TYPE_UINT8_T, 0, 23, offsetof(mavlink_cc_ai_diagnostic_t, severity) }, \
         { "confidence_percent", NULL, MAVLINK_TYPE_UINT8_T, 0, 24, offsetof(mavlink_cc_ai_diagnostic_t, confidence_percent) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 25, offsetof(mavlink_cc_ai_diagnostic_t, schema_version) }, \
         } \
}
#endif

/**
 * @brief Pack a cc_ai_diagnostic message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 *
 * @param companion_timestamp_us [us] CC monotonic time at finding creation.
 * @param sequence  Diagnostic sequence, monotonic per cc_boot_id.
 * @param value  Measured value that triggered the finding.
 * @param limit  Threshold/limit the value was compared against.
 * @param detail_code  Evidence code (same namespace as CC_HEALTH_REPORT.detail_code).
 * @param subsystem  Subsystem of the finding.
 * @param severity  Severity of this individual finding.
 * @param confidence_percent [%] Confidence [0..100].
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_ai_diagnostic_pack(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg,
                               uint64_t companion_timestamp_us, uint32_t sequence, float value, float limit, uint16_t detail_code, uint8_t subsystem, uint8_t severity, uint8_t confidence_percent, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN];
    _mav_put_uint64_t(buf, 0, companion_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_float(buf, 12, value);
    _mav_put_float(buf, 16, limit);
    _mav_put_uint16_t(buf, 20, detail_code);
    _mav_put_uint8_t(buf, 22, subsystem);
    _mav_put_uint8_t(buf, 23, severity);
    _mav_put_uint8_t(buf, 24, confidence_percent);
    _mav_put_uint8_t(buf, 25, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN);
#else
    mavlink_cc_ai_diagnostic_t packet;
    packet.companion_timestamp_us = companion_timestamp_us;
    packet.sequence = sequence;
    packet.value = value;
    packet.limit = limit;
    packet.detail_code = detail_code;
    packet.subsystem = subsystem;
    packet.severity = severity;
    packet.confidence_percent = confidence_percent;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC;
    return mavlink_finalize_message(msg, system_id, component_id, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_MIN_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_CRC);
}

/**
 * @brief Pack a cc_ai_diagnostic message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 *
 * @param companion_timestamp_us [us] CC monotonic time at finding creation.
 * @param sequence  Diagnostic sequence, monotonic per cc_boot_id.
 * @param value  Measured value that triggered the finding.
 * @param limit  Threshold/limit the value was compared against.
 * @param detail_code  Evidence code (same namespace as CC_HEALTH_REPORT.detail_code).
 * @param subsystem  Subsystem of the finding.
 * @param severity  Severity of this individual finding.
 * @param confidence_percent [%] Confidence [0..100].
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_ai_diagnostic_pack_status(uint8_t system_id, uint8_t component_id, mavlink_status_t *_status, mavlink_message_t* msg,
                               uint64_t companion_timestamp_us, uint32_t sequence, float value, float limit, uint16_t detail_code, uint8_t subsystem, uint8_t severity, uint8_t confidence_percent, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN];
    _mav_put_uint64_t(buf, 0, companion_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_float(buf, 12, value);
    _mav_put_float(buf, 16, limit);
    _mav_put_uint16_t(buf, 20, detail_code);
    _mav_put_uint8_t(buf, 22, subsystem);
    _mav_put_uint8_t(buf, 23, severity);
    _mav_put_uint8_t(buf, 24, confidence_percent);
    _mav_put_uint8_t(buf, 25, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN);
#else
    mavlink_cc_ai_diagnostic_t packet;
    packet.companion_timestamp_us = companion_timestamp_us;
    packet.sequence = sequence;
    packet.value = value;
    packet.limit = limit;
    packet.detail_code = detail_code;
    packet.subsystem = subsystem;
    packet.severity = severity;
    packet.confidence_percent = confidence_percent;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC;
#if MAVLINK_CRC_EXTRA
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_MIN_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_CRC);
#else
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_MIN_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN);
#endif
}

/**
 * @brief Pack a cc_ai_diagnostic message on a channel
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param companion_timestamp_us [us] CC monotonic time at finding creation.
 * @param sequence  Diagnostic sequence, monotonic per cc_boot_id.
 * @param value  Measured value that triggered the finding.
 * @param limit  Threshold/limit the value was compared against.
 * @param detail_code  Evidence code (same namespace as CC_HEALTH_REPORT.detail_code).
 * @param subsystem  Subsystem of the finding.
 * @param severity  Severity of this individual finding.
 * @param confidence_percent [%] Confidence [0..100].
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_ai_diagnostic_pack_chan(uint8_t system_id, uint8_t component_id, uint8_t chan,
                               mavlink_message_t* msg,
                                   uint64_t companion_timestamp_us,uint32_t sequence,float value,float limit,uint16_t detail_code,uint8_t subsystem,uint8_t severity,uint8_t confidence_percent,uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN];
    _mav_put_uint64_t(buf, 0, companion_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_float(buf, 12, value);
    _mav_put_float(buf, 16, limit);
    _mav_put_uint16_t(buf, 20, detail_code);
    _mav_put_uint8_t(buf, 22, subsystem);
    _mav_put_uint8_t(buf, 23, severity);
    _mav_put_uint8_t(buf, 24, confidence_percent);
    _mav_put_uint8_t(buf, 25, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN);
#else
    mavlink_cc_ai_diagnostic_t packet;
    packet.companion_timestamp_us = companion_timestamp_us;
    packet.sequence = sequence;
    packet.value = value;
    packet.limit = limit;
    packet.detail_code = detail_code;
    packet.subsystem = subsystem;
    packet.severity = severity;
    packet.confidence_percent = confidence_percent;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC;
    return mavlink_finalize_message_chan(msg, system_id, component_id, chan, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_MIN_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_CRC);
}

/**
 * @brief Encode a cc_ai_diagnostic struct
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 * @param cc_ai_diagnostic C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_ai_diagnostic_encode(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg, const mavlink_cc_ai_diagnostic_t* cc_ai_diagnostic)
{
    return mavlink_msg_cc_ai_diagnostic_pack(system_id, component_id, msg, cc_ai_diagnostic->companion_timestamp_us, cc_ai_diagnostic->sequence, cc_ai_diagnostic->value, cc_ai_diagnostic->limit, cc_ai_diagnostic->detail_code, cc_ai_diagnostic->subsystem, cc_ai_diagnostic->severity, cc_ai_diagnostic->confidence_percent, cc_ai_diagnostic->schema_version);
}

/**
 * @brief Encode a cc_ai_diagnostic struct on a channel
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param cc_ai_diagnostic C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_ai_diagnostic_encode_chan(uint8_t system_id, uint8_t component_id, uint8_t chan, mavlink_message_t* msg, const mavlink_cc_ai_diagnostic_t* cc_ai_diagnostic)
{
    return mavlink_msg_cc_ai_diagnostic_pack_chan(system_id, component_id, chan, msg, cc_ai_diagnostic->companion_timestamp_us, cc_ai_diagnostic->sequence, cc_ai_diagnostic->value, cc_ai_diagnostic->limit, cc_ai_diagnostic->detail_code, cc_ai_diagnostic->subsystem, cc_ai_diagnostic->severity, cc_ai_diagnostic->confidence_percent, cc_ai_diagnostic->schema_version);
}

/**
 * @brief Encode a cc_ai_diagnostic struct with provided status structure
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 * @param cc_ai_diagnostic C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_ai_diagnostic_encode_status(uint8_t system_id, uint8_t component_id, mavlink_status_t* _status, mavlink_message_t* msg, const mavlink_cc_ai_diagnostic_t* cc_ai_diagnostic)
{
    return mavlink_msg_cc_ai_diagnostic_pack_status(system_id, component_id, _status, msg,  cc_ai_diagnostic->companion_timestamp_us, cc_ai_diagnostic->sequence, cc_ai_diagnostic->value, cc_ai_diagnostic->limit, cc_ai_diagnostic->detail_code, cc_ai_diagnostic->subsystem, cc_ai_diagnostic->severity, cc_ai_diagnostic->confidence_percent, cc_ai_diagnostic->schema_version);
}

/**
 * @brief Send a cc_ai_diagnostic message
 * @param chan MAVLink channel to send the message
 *
 * @param companion_timestamp_us [us] CC monotonic time at finding creation.
 * @param sequence  Diagnostic sequence, monotonic per cc_boot_id.
 * @param value  Measured value that triggered the finding.
 * @param limit  Threshold/limit the value was compared against.
 * @param detail_code  Evidence code (same namespace as CC_HEALTH_REPORT.detail_code).
 * @param subsystem  Subsystem of the finding.
 * @param severity  Severity of this individual finding.
 * @param confidence_percent [%] Confidence [0..100].
 * @param schema_version  Payload schema version.
 */
#ifdef MAVLINK_USE_CONVENIENCE_FUNCTIONS

static inline void mavlink_msg_cc_ai_diagnostic_send(mavlink_channel_t chan, uint64_t companion_timestamp_us, uint32_t sequence, float value, float limit, uint16_t detail_code, uint8_t subsystem, uint8_t severity, uint8_t confidence_percent, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN];
    _mav_put_uint64_t(buf, 0, companion_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_float(buf, 12, value);
    _mav_put_float(buf, 16, limit);
    _mav_put_uint16_t(buf, 20, detail_code);
    _mav_put_uint8_t(buf, 22, subsystem);
    _mav_put_uint8_t(buf, 23, severity);
    _mav_put_uint8_t(buf, 24, confidence_percent);
    _mav_put_uint8_t(buf, 25, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC, buf, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_MIN_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_CRC);
#else
    mavlink_cc_ai_diagnostic_t packet;
    packet.companion_timestamp_us = companion_timestamp_us;
    packet.sequence = sequence;
    packet.value = value;
    packet.limit = limit;
    packet.detail_code = detail_code;
    packet.subsystem = subsystem;
    packet.severity = severity;
    packet.confidence_percent = confidence_percent;
    packet.schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC, (const char *)&packet, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_MIN_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_CRC);
#endif
}

/**
 * @brief Send a cc_ai_diagnostic message
 * @param chan MAVLink channel to send the message
 * @param struct The MAVLink struct to serialize
 */
static inline void mavlink_msg_cc_ai_diagnostic_send_struct(mavlink_channel_t chan, const mavlink_cc_ai_diagnostic_t* cc_ai_diagnostic)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    mavlink_msg_cc_ai_diagnostic_send(chan, cc_ai_diagnostic->companion_timestamp_us, cc_ai_diagnostic->sequence, cc_ai_diagnostic->value, cc_ai_diagnostic->limit, cc_ai_diagnostic->detail_code, cc_ai_diagnostic->subsystem, cc_ai_diagnostic->severity, cc_ai_diagnostic->confidence_percent, cc_ai_diagnostic->schema_version);
#else
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC, (const char *)cc_ai_diagnostic, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_MIN_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_CRC);
#endif
}

#if MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN <= MAVLINK_MAX_PAYLOAD_LEN
/*
  This variant of _send() can be used to save stack space by reusing
  memory from the receive buffer.  The caller provides a
  mavlink_message_t which is the size of a full mavlink message. This
  is usually the receive buffer for the channel, and allows a reply to an
  incoming message with minimum stack space usage.
 */
static inline void mavlink_msg_cc_ai_diagnostic_send_buf(mavlink_message_t *msgbuf, mavlink_channel_t chan,  uint64_t companion_timestamp_us, uint32_t sequence, float value, float limit, uint16_t detail_code, uint8_t subsystem, uint8_t severity, uint8_t confidence_percent, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char *buf = (char *)msgbuf;
    _mav_put_uint64_t(buf, 0, companion_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_float(buf, 12, value);
    _mav_put_float(buf, 16, limit);
    _mav_put_uint16_t(buf, 20, detail_code);
    _mav_put_uint8_t(buf, 22, subsystem);
    _mav_put_uint8_t(buf, 23, severity);
    _mav_put_uint8_t(buf, 24, confidence_percent);
    _mav_put_uint8_t(buf, 25, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC, buf, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_MIN_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_CRC);
#else
    mavlink_cc_ai_diagnostic_t *packet = (mavlink_cc_ai_diagnostic_t *)msgbuf;
    packet->companion_timestamp_us = companion_timestamp_us;
    packet->sequence = sequence;
    packet->value = value;
    packet->limit = limit;
    packet->detail_code = detail_code;
    packet->subsystem = subsystem;
    packet->severity = severity;
    packet->confidence_percent = confidence_percent;
    packet->schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC, (const char *)packet, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_MIN_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_CRC);
#endif
}
#endif

#endif

// MESSAGE CC_AI_DIAGNOSTIC UNPACKING


/**
 * @brief Get field companion_timestamp_us from cc_ai_diagnostic message
 *
 * @return [us] CC monotonic time at finding creation.
 */
static inline uint64_t mavlink_msg_cc_ai_diagnostic_get_companion_timestamp_us(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint64_t(msg,  0);
}

/**
 * @brief Get field sequence from cc_ai_diagnostic message
 *
 * @return  Diagnostic sequence, monotonic per cc_boot_id.
 */
static inline uint32_t mavlink_msg_cc_ai_diagnostic_get_sequence(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  8);
}

/**
 * @brief Get field value from cc_ai_diagnostic message
 *
 * @return  Measured value that triggered the finding.
 */
static inline float mavlink_msg_cc_ai_diagnostic_get_value(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  12);
}

/**
 * @brief Get field limit from cc_ai_diagnostic message
 *
 * @return  Threshold/limit the value was compared against.
 */
static inline float mavlink_msg_cc_ai_diagnostic_get_limit(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  16);
}

/**
 * @brief Get field detail_code from cc_ai_diagnostic message
 *
 * @return  Evidence code (same namespace as CC_HEALTH_REPORT.detail_code).
 */
static inline uint16_t mavlink_msg_cc_ai_diagnostic_get_detail_code(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint16_t(msg,  20);
}

/**
 * @brief Get field subsystem from cc_ai_diagnostic message
 *
 * @return  Subsystem of the finding.
 */
static inline uint8_t mavlink_msg_cc_ai_diagnostic_get_subsystem(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  22);
}

/**
 * @brief Get field severity from cc_ai_diagnostic message
 *
 * @return  Severity of this individual finding.
 */
static inline uint8_t mavlink_msg_cc_ai_diagnostic_get_severity(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  23);
}

/**
 * @brief Get field confidence_percent from cc_ai_diagnostic message
 *
 * @return [%] Confidence [0..100].
 */
static inline uint8_t mavlink_msg_cc_ai_diagnostic_get_confidence_percent(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  24);
}

/**
 * @brief Get field schema_version from cc_ai_diagnostic message
 *
 * @return  Payload schema version.
 */
static inline uint8_t mavlink_msg_cc_ai_diagnostic_get_schema_version(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  25);
}

/**
 * @brief Decode a cc_ai_diagnostic message into a struct
 *
 * @param msg The message to decode
 * @param cc_ai_diagnostic C-struct to decode the message contents into
 */
static inline void mavlink_msg_cc_ai_diagnostic_decode(const mavlink_message_t* msg, mavlink_cc_ai_diagnostic_t* cc_ai_diagnostic)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    cc_ai_diagnostic->companion_timestamp_us = mavlink_msg_cc_ai_diagnostic_get_companion_timestamp_us(msg);
    cc_ai_diagnostic->sequence = mavlink_msg_cc_ai_diagnostic_get_sequence(msg);
    cc_ai_diagnostic->value = mavlink_msg_cc_ai_diagnostic_get_value(msg);
    cc_ai_diagnostic->limit = mavlink_msg_cc_ai_diagnostic_get_limit(msg);
    cc_ai_diagnostic->detail_code = mavlink_msg_cc_ai_diagnostic_get_detail_code(msg);
    cc_ai_diagnostic->subsystem = mavlink_msg_cc_ai_diagnostic_get_subsystem(msg);
    cc_ai_diagnostic->severity = mavlink_msg_cc_ai_diagnostic_get_severity(msg);
    cc_ai_diagnostic->confidence_percent = mavlink_msg_cc_ai_diagnostic_get_confidence_percent(msg);
    cc_ai_diagnostic->schema_version = mavlink_msg_cc_ai_diagnostic_get_schema_version(msg);
#else
        uint8_t len = msg->len < MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN? msg->len : MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN;
        memset(cc_ai_diagnostic, 0, MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_LEN);
    memcpy(cc_ai_diagnostic, _MAV_PAYLOAD(msg), len);
#endif
}
