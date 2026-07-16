#pragma once
// MESSAGE CC_SAFETY_STATUS PACKING

#define MAVLINK_MSG_ID_CC_SAFETY_STATUS 54007


typedef struct __mavlink_cc_safety_status_t {
 uint64_t fc_timestamp_us; /*< [us] FC monotonic time since PX4 boot.*/
 uint32_t last_report_sequence; /*<  Highest CC_HEALTH_REPORT sequence accepted by the monitor.*/
 uint32_t active_health_flags; /*<  Health flags of the last accepted report.*/
 uint32_t report_age_ms; /*< [ms] Age of the last accepted report.*/
 uint32_t missed_reports; /*<  Cumulative sequence gaps detected since boot.*/
 uint8_t companion_state; /*<  Monitor-judged companion state.*/
 uint8_t action_taken; /*<  Action the monitor actually executed (may differ from recommendation; parameter policy wins).*/
 uint8_t reject_reason; /*<  Reason the most recent inbound message was rejected (CC_REJECT_NONE if none).*/
 uint8_t schema_version; /*<  Payload schema version.*/
} mavlink_cc_safety_status_t;

#define MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN 28
#define MAVLINK_MSG_ID_CC_SAFETY_STATUS_MIN_LEN 28
#define MAVLINK_MSG_ID_54007_LEN 28
#define MAVLINK_MSG_ID_54007_MIN_LEN 28

#define MAVLINK_MSG_ID_CC_SAFETY_STATUS_CRC 93
#define MAVLINK_MSG_ID_54007_CRC 93



#if MAVLINK_COMMAND_24BIT
#define MAVLINK_MESSAGE_INFO_CC_SAFETY_STATUS { \
    54007, \
    "CC_SAFETY_STATUS", \
    9, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_safety_status_t, fc_timestamp_us) }, \
         { "last_report_sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_safety_status_t, last_report_sequence) }, \
         { "active_health_flags", NULL, MAVLINK_TYPE_UINT32_T, 0, 12, offsetof(mavlink_cc_safety_status_t, active_health_flags) }, \
         { "report_age_ms", NULL, MAVLINK_TYPE_UINT32_T, 0, 16, offsetof(mavlink_cc_safety_status_t, report_age_ms) }, \
         { "missed_reports", NULL, MAVLINK_TYPE_UINT32_T, 0, 20, offsetof(mavlink_cc_safety_status_t, missed_reports) }, \
         { "companion_state", NULL, MAVLINK_TYPE_UINT8_T, 0, 24, offsetof(mavlink_cc_safety_status_t, companion_state) }, \
         { "action_taken", NULL, MAVLINK_TYPE_UINT8_T, 0, 25, offsetof(mavlink_cc_safety_status_t, action_taken) }, \
         { "reject_reason", NULL, MAVLINK_TYPE_UINT8_T, 0, 26, offsetof(mavlink_cc_safety_status_t, reject_reason) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 27, offsetof(mavlink_cc_safety_status_t, schema_version) }, \
         } \
}
#else
#define MAVLINK_MESSAGE_INFO_CC_SAFETY_STATUS { \
    "CC_SAFETY_STATUS", \
    9, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_safety_status_t, fc_timestamp_us) }, \
         { "last_report_sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_safety_status_t, last_report_sequence) }, \
         { "active_health_flags", NULL, MAVLINK_TYPE_UINT32_T, 0, 12, offsetof(mavlink_cc_safety_status_t, active_health_flags) }, \
         { "report_age_ms", NULL, MAVLINK_TYPE_UINT32_T, 0, 16, offsetof(mavlink_cc_safety_status_t, report_age_ms) }, \
         { "missed_reports", NULL, MAVLINK_TYPE_UINT32_T, 0, 20, offsetof(mavlink_cc_safety_status_t, missed_reports) }, \
         { "companion_state", NULL, MAVLINK_TYPE_UINT8_T, 0, 24, offsetof(mavlink_cc_safety_status_t, companion_state) }, \
         { "action_taken", NULL, MAVLINK_TYPE_UINT8_T, 0, 25, offsetof(mavlink_cc_safety_status_t, action_taken) }, \
         { "reject_reason", NULL, MAVLINK_TYPE_UINT8_T, 0, 26, offsetof(mavlink_cc_safety_status_t, reject_reason) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 27, offsetof(mavlink_cc_safety_status_t, schema_version) }, \
         } \
}
#endif

/**
 * @brief Pack a cc_safety_status message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param last_report_sequence  Highest CC_HEALTH_REPORT sequence accepted by the monitor.
 * @param active_health_flags  Health flags of the last accepted report.
 * @param report_age_ms [ms] Age of the last accepted report.
 * @param missed_reports  Cumulative sequence gaps detected since boot.
 * @param companion_state  Monitor-judged companion state.
 * @param action_taken  Action the monitor actually executed (may differ from recommendation; parameter policy wins).
 * @param reject_reason  Reason the most recent inbound message was rejected (CC_REJECT_NONE if none).
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_safety_status_pack(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t last_report_sequence, uint32_t active_health_flags, uint32_t report_age_ms, uint32_t missed_reports, uint8_t companion_state, uint8_t action_taken, uint8_t reject_reason, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, last_report_sequence);
    _mav_put_uint32_t(buf, 12, active_health_flags);
    _mav_put_uint32_t(buf, 16, report_age_ms);
    _mav_put_uint32_t(buf, 20, missed_reports);
    _mav_put_uint8_t(buf, 24, companion_state);
    _mav_put_uint8_t(buf, 25, action_taken);
    _mav_put_uint8_t(buf, 26, reject_reason);
    _mav_put_uint8_t(buf, 27, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN);
#else
    mavlink_cc_safety_status_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.last_report_sequence = last_report_sequence;
    packet.active_health_flags = active_health_flags;
    packet.report_age_ms = report_age_ms;
    packet.missed_reports = missed_reports;
    packet.companion_state = companion_state;
    packet.action_taken = action_taken;
    packet.reject_reason = reject_reason;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_SAFETY_STATUS;
    return mavlink_finalize_message(msg, system_id, component_id, MAVLINK_MSG_ID_CC_SAFETY_STATUS_MIN_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_CRC);
}

/**
 * @brief Pack a cc_safety_status message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param last_report_sequence  Highest CC_HEALTH_REPORT sequence accepted by the monitor.
 * @param active_health_flags  Health flags of the last accepted report.
 * @param report_age_ms [ms] Age of the last accepted report.
 * @param missed_reports  Cumulative sequence gaps detected since boot.
 * @param companion_state  Monitor-judged companion state.
 * @param action_taken  Action the monitor actually executed (may differ from recommendation; parameter policy wins).
 * @param reject_reason  Reason the most recent inbound message was rejected (CC_REJECT_NONE if none).
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_safety_status_pack_status(uint8_t system_id, uint8_t component_id, mavlink_status_t *_status, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t last_report_sequence, uint32_t active_health_flags, uint32_t report_age_ms, uint32_t missed_reports, uint8_t companion_state, uint8_t action_taken, uint8_t reject_reason, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, last_report_sequence);
    _mav_put_uint32_t(buf, 12, active_health_flags);
    _mav_put_uint32_t(buf, 16, report_age_ms);
    _mav_put_uint32_t(buf, 20, missed_reports);
    _mav_put_uint8_t(buf, 24, companion_state);
    _mav_put_uint8_t(buf, 25, action_taken);
    _mav_put_uint8_t(buf, 26, reject_reason);
    _mav_put_uint8_t(buf, 27, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN);
#else
    mavlink_cc_safety_status_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.last_report_sequence = last_report_sequence;
    packet.active_health_flags = active_health_flags;
    packet.report_age_ms = report_age_ms;
    packet.missed_reports = missed_reports;
    packet.companion_state = companion_state;
    packet.action_taken = action_taken;
    packet.reject_reason = reject_reason;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_SAFETY_STATUS;
#if MAVLINK_CRC_EXTRA
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_SAFETY_STATUS_MIN_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_CRC);
#else
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_SAFETY_STATUS_MIN_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN);
#endif
}

/**
 * @brief Pack a cc_safety_status message on a channel
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param last_report_sequence  Highest CC_HEALTH_REPORT sequence accepted by the monitor.
 * @param active_health_flags  Health flags of the last accepted report.
 * @param report_age_ms [ms] Age of the last accepted report.
 * @param missed_reports  Cumulative sequence gaps detected since boot.
 * @param companion_state  Monitor-judged companion state.
 * @param action_taken  Action the monitor actually executed (may differ from recommendation; parameter policy wins).
 * @param reject_reason  Reason the most recent inbound message was rejected (CC_REJECT_NONE if none).
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_safety_status_pack_chan(uint8_t system_id, uint8_t component_id, uint8_t chan,
                               mavlink_message_t* msg,
                                   uint64_t fc_timestamp_us,uint32_t last_report_sequence,uint32_t active_health_flags,uint32_t report_age_ms,uint32_t missed_reports,uint8_t companion_state,uint8_t action_taken,uint8_t reject_reason,uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, last_report_sequence);
    _mav_put_uint32_t(buf, 12, active_health_flags);
    _mav_put_uint32_t(buf, 16, report_age_ms);
    _mav_put_uint32_t(buf, 20, missed_reports);
    _mav_put_uint8_t(buf, 24, companion_state);
    _mav_put_uint8_t(buf, 25, action_taken);
    _mav_put_uint8_t(buf, 26, reject_reason);
    _mav_put_uint8_t(buf, 27, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN);
#else
    mavlink_cc_safety_status_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.last_report_sequence = last_report_sequence;
    packet.active_health_flags = active_health_flags;
    packet.report_age_ms = report_age_ms;
    packet.missed_reports = missed_reports;
    packet.companion_state = companion_state;
    packet.action_taken = action_taken;
    packet.reject_reason = reject_reason;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_SAFETY_STATUS;
    return mavlink_finalize_message_chan(msg, system_id, component_id, chan, MAVLINK_MSG_ID_CC_SAFETY_STATUS_MIN_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_CRC);
}

/**
 * @brief Encode a cc_safety_status struct
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 * @param cc_safety_status C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_safety_status_encode(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg, const mavlink_cc_safety_status_t* cc_safety_status)
{
    return mavlink_msg_cc_safety_status_pack(system_id, component_id, msg, cc_safety_status->fc_timestamp_us, cc_safety_status->last_report_sequence, cc_safety_status->active_health_flags, cc_safety_status->report_age_ms, cc_safety_status->missed_reports, cc_safety_status->companion_state, cc_safety_status->action_taken, cc_safety_status->reject_reason, cc_safety_status->schema_version);
}

/**
 * @brief Encode a cc_safety_status struct on a channel
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param cc_safety_status C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_safety_status_encode_chan(uint8_t system_id, uint8_t component_id, uint8_t chan, mavlink_message_t* msg, const mavlink_cc_safety_status_t* cc_safety_status)
{
    return mavlink_msg_cc_safety_status_pack_chan(system_id, component_id, chan, msg, cc_safety_status->fc_timestamp_us, cc_safety_status->last_report_sequence, cc_safety_status->active_health_flags, cc_safety_status->report_age_ms, cc_safety_status->missed_reports, cc_safety_status->companion_state, cc_safety_status->action_taken, cc_safety_status->reject_reason, cc_safety_status->schema_version);
}

/**
 * @brief Encode a cc_safety_status struct with provided status structure
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 * @param cc_safety_status C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_safety_status_encode_status(uint8_t system_id, uint8_t component_id, mavlink_status_t* _status, mavlink_message_t* msg, const mavlink_cc_safety_status_t* cc_safety_status)
{
    return mavlink_msg_cc_safety_status_pack_status(system_id, component_id, _status, msg,  cc_safety_status->fc_timestamp_us, cc_safety_status->last_report_sequence, cc_safety_status->active_health_flags, cc_safety_status->report_age_ms, cc_safety_status->missed_reports, cc_safety_status->companion_state, cc_safety_status->action_taken, cc_safety_status->reject_reason, cc_safety_status->schema_version);
}

/**
 * @brief Send a cc_safety_status message
 * @param chan MAVLink channel to send the message
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param last_report_sequence  Highest CC_HEALTH_REPORT sequence accepted by the monitor.
 * @param active_health_flags  Health flags of the last accepted report.
 * @param report_age_ms [ms] Age of the last accepted report.
 * @param missed_reports  Cumulative sequence gaps detected since boot.
 * @param companion_state  Monitor-judged companion state.
 * @param action_taken  Action the monitor actually executed (may differ from recommendation; parameter policy wins).
 * @param reject_reason  Reason the most recent inbound message was rejected (CC_REJECT_NONE if none).
 * @param schema_version  Payload schema version.
 */
#ifdef MAVLINK_USE_CONVENIENCE_FUNCTIONS

static inline void mavlink_msg_cc_safety_status_send(mavlink_channel_t chan, uint64_t fc_timestamp_us, uint32_t last_report_sequence, uint32_t active_health_flags, uint32_t report_age_ms, uint32_t missed_reports, uint8_t companion_state, uint8_t action_taken, uint8_t reject_reason, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, last_report_sequence);
    _mav_put_uint32_t(buf, 12, active_health_flags);
    _mav_put_uint32_t(buf, 16, report_age_ms);
    _mav_put_uint32_t(buf, 20, missed_reports);
    _mav_put_uint8_t(buf, 24, companion_state);
    _mav_put_uint8_t(buf, 25, action_taken);
    _mav_put_uint8_t(buf, 26, reject_reason);
    _mav_put_uint8_t(buf, 27, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_SAFETY_STATUS, buf, MAVLINK_MSG_ID_CC_SAFETY_STATUS_MIN_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_CRC);
#else
    mavlink_cc_safety_status_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.last_report_sequence = last_report_sequence;
    packet.active_health_flags = active_health_flags;
    packet.report_age_ms = report_age_ms;
    packet.missed_reports = missed_reports;
    packet.companion_state = companion_state;
    packet.action_taken = action_taken;
    packet.reject_reason = reject_reason;
    packet.schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_SAFETY_STATUS, (const char *)&packet, MAVLINK_MSG_ID_CC_SAFETY_STATUS_MIN_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_CRC);
#endif
}

/**
 * @brief Send a cc_safety_status message
 * @param chan MAVLink channel to send the message
 * @param struct The MAVLink struct to serialize
 */
static inline void mavlink_msg_cc_safety_status_send_struct(mavlink_channel_t chan, const mavlink_cc_safety_status_t* cc_safety_status)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    mavlink_msg_cc_safety_status_send(chan, cc_safety_status->fc_timestamp_us, cc_safety_status->last_report_sequence, cc_safety_status->active_health_flags, cc_safety_status->report_age_ms, cc_safety_status->missed_reports, cc_safety_status->companion_state, cc_safety_status->action_taken, cc_safety_status->reject_reason, cc_safety_status->schema_version);
#else
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_SAFETY_STATUS, (const char *)cc_safety_status, MAVLINK_MSG_ID_CC_SAFETY_STATUS_MIN_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_CRC);
#endif
}

#if MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN <= MAVLINK_MAX_PAYLOAD_LEN
/*
  This variant of _send() can be used to save stack space by reusing
  memory from the receive buffer.  The caller provides a
  mavlink_message_t which is the size of a full mavlink message. This
  is usually the receive buffer for the channel, and allows a reply to an
  incoming message with minimum stack space usage.
 */
static inline void mavlink_msg_cc_safety_status_send_buf(mavlink_message_t *msgbuf, mavlink_channel_t chan,  uint64_t fc_timestamp_us, uint32_t last_report_sequence, uint32_t active_health_flags, uint32_t report_age_ms, uint32_t missed_reports, uint8_t companion_state, uint8_t action_taken, uint8_t reject_reason, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char *buf = (char *)msgbuf;
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, last_report_sequence);
    _mav_put_uint32_t(buf, 12, active_health_flags);
    _mav_put_uint32_t(buf, 16, report_age_ms);
    _mav_put_uint32_t(buf, 20, missed_reports);
    _mav_put_uint8_t(buf, 24, companion_state);
    _mav_put_uint8_t(buf, 25, action_taken);
    _mav_put_uint8_t(buf, 26, reject_reason);
    _mav_put_uint8_t(buf, 27, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_SAFETY_STATUS, buf, MAVLINK_MSG_ID_CC_SAFETY_STATUS_MIN_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_CRC);
#else
    mavlink_cc_safety_status_t *packet = (mavlink_cc_safety_status_t *)msgbuf;
    packet->fc_timestamp_us = fc_timestamp_us;
    packet->last_report_sequence = last_report_sequence;
    packet->active_health_flags = active_health_flags;
    packet->report_age_ms = report_age_ms;
    packet->missed_reports = missed_reports;
    packet->companion_state = companion_state;
    packet->action_taken = action_taken;
    packet->reject_reason = reject_reason;
    packet->schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_SAFETY_STATUS, (const char *)packet, MAVLINK_MSG_ID_CC_SAFETY_STATUS_MIN_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN, MAVLINK_MSG_ID_CC_SAFETY_STATUS_CRC);
#endif
}
#endif

#endif

// MESSAGE CC_SAFETY_STATUS UNPACKING


/**
 * @brief Get field fc_timestamp_us from cc_safety_status message
 *
 * @return [us] FC monotonic time since PX4 boot.
 */
static inline uint64_t mavlink_msg_cc_safety_status_get_fc_timestamp_us(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint64_t(msg,  0);
}

/**
 * @brief Get field last_report_sequence from cc_safety_status message
 *
 * @return  Highest CC_HEALTH_REPORT sequence accepted by the monitor.
 */
static inline uint32_t mavlink_msg_cc_safety_status_get_last_report_sequence(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  8);
}

/**
 * @brief Get field active_health_flags from cc_safety_status message
 *
 * @return  Health flags of the last accepted report.
 */
static inline uint32_t mavlink_msg_cc_safety_status_get_active_health_flags(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  12);
}

/**
 * @brief Get field report_age_ms from cc_safety_status message
 *
 * @return [ms] Age of the last accepted report.
 */
static inline uint32_t mavlink_msg_cc_safety_status_get_report_age_ms(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  16);
}

/**
 * @brief Get field missed_reports from cc_safety_status message
 *
 * @return  Cumulative sequence gaps detected since boot.
 */
static inline uint32_t mavlink_msg_cc_safety_status_get_missed_reports(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  20);
}

/**
 * @brief Get field companion_state from cc_safety_status message
 *
 * @return  Monitor-judged companion state.
 */
static inline uint8_t mavlink_msg_cc_safety_status_get_companion_state(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  24);
}

/**
 * @brief Get field action_taken from cc_safety_status message
 *
 * @return  Action the monitor actually executed (may differ from recommendation; parameter policy wins).
 */
static inline uint8_t mavlink_msg_cc_safety_status_get_action_taken(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  25);
}

/**
 * @brief Get field reject_reason from cc_safety_status message
 *
 * @return  Reason the most recent inbound message was rejected (CC_REJECT_NONE if none).
 */
static inline uint8_t mavlink_msg_cc_safety_status_get_reject_reason(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  26);
}

/**
 * @brief Get field schema_version from cc_safety_status message
 *
 * @return  Payload schema version.
 */
static inline uint8_t mavlink_msg_cc_safety_status_get_schema_version(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  27);
}

/**
 * @brief Decode a cc_safety_status message into a struct
 *
 * @param msg The message to decode
 * @param cc_safety_status C-struct to decode the message contents into
 */
static inline void mavlink_msg_cc_safety_status_decode(const mavlink_message_t* msg, mavlink_cc_safety_status_t* cc_safety_status)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    cc_safety_status->fc_timestamp_us = mavlink_msg_cc_safety_status_get_fc_timestamp_us(msg);
    cc_safety_status->last_report_sequence = mavlink_msg_cc_safety_status_get_last_report_sequence(msg);
    cc_safety_status->active_health_flags = mavlink_msg_cc_safety_status_get_active_health_flags(msg);
    cc_safety_status->report_age_ms = mavlink_msg_cc_safety_status_get_report_age_ms(msg);
    cc_safety_status->missed_reports = mavlink_msg_cc_safety_status_get_missed_reports(msg);
    cc_safety_status->companion_state = mavlink_msg_cc_safety_status_get_companion_state(msg);
    cc_safety_status->action_taken = mavlink_msg_cc_safety_status_get_action_taken(msg);
    cc_safety_status->reject_reason = mavlink_msg_cc_safety_status_get_reject_reason(msg);
    cc_safety_status->schema_version = mavlink_msg_cc_safety_status_get_schema_version(msg);
#else
        uint8_t len = msg->len < MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN? msg->len : MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN;
        memset(cc_safety_status, 0, MAVLINK_MSG_ID_CC_SAFETY_STATUS_LEN);
    memcpy(cc_safety_status, _MAV_PAYLOAD(msg), len);
#endif
}
