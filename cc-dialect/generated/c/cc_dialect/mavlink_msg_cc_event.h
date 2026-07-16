#pragma once
// MESSAGE CC_EVENT PACKING

#define MAVLINK_MSG_ID_CC_EVENT 54006


typedef struct __mavlink_cc_event_t {
 uint64_t fc_timestamp_us; /*< [us] FC monotonic time since PX4 boot.*/
 uint32_t sequence; /*<  Per-stream monotonic counter.*/
 uint32_t event_id; /*<  Event identifier (PX4 events namespace or CC-defined block).*/
 uint32_t argument0; /*<  Event argument 0 (meaning defined per event_id).*/
 uint32_t argument1; /*<  Event argument 1 (meaning defined per event_id).*/
 uint8_t severity; /*<  Event severity.*/
 uint8_t subsystem; /*<  Related subsystem.*/
 uint8_t schema_version; /*<  Payload schema version.*/
} mavlink_cc_event_t;

#define MAVLINK_MSG_ID_CC_EVENT_LEN 27
#define MAVLINK_MSG_ID_CC_EVENT_MIN_LEN 27
#define MAVLINK_MSG_ID_54006_LEN 27
#define MAVLINK_MSG_ID_54006_MIN_LEN 27

#define MAVLINK_MSG_ID_CC_EVENT_CRC 11
#define MAVLINK_MSG_ID_54006_CRC 11



#if MAVLINK_COMMAND_24BIT
#define MAVLINK_MESSAGE_INFO_CC_EVENT { \
    54006, \
    "CC_EVENT", \
    8, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_event_t, fc_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_event_t, sequence) }, \
         { "event_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 12, offsetof(mavlink_cc_event_t, event_id) }, \
         { "argument0", NULL, MAVLINK_TYPE_UINT32_T, 0, 16, offsetof(mavlink_cc_event_t, argument0) }, \
         { "argument1", NULL, MAVLINK_TYPE_UINT32_T, 0, 20, offsetof(mavlink_cc_event_t, argument1) }, \
         { "severity", NULL, MAVLINK_TYPE_UINT8_T, 0, 24, offsetof(mavlink_cc_event_t, severity) }, \
         { "subsystem", NULL, MAVLINK_TYPE_UINT8_T, 0, 25, offsetof(mavlink_cc_event_t, subsystem) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 26, offsetof(mavlink_cc_event_t, schema_version) }, \
         } \
}
#else
#define MAVLINK_MESSAGE_INFO_CC_EVENT { \
    "CC_EVENT", \
    8, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_event_t, fc_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_event_t, sequence) }, \
         { "event_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 12, offsetof(mavlink_cc_event_t, event_id) }, \
         { "argument0", NULL, MAVLINK_TYPE_UINT32_T, 0, 16, offsetof(mavlink_cc_event_t, argument0) }, \
         { "argument1", NULL, MAVLINK_TYPE_UINT32_T, 0, 20, offsetof(mavlink_cc_event_t, argument1) }, \
         { "severity", NULL, MAVLINK_TYPE_UINT8_T, 0, 24, offsetof(mavlink_cc_event_t, severity) }, \
         { "subsystem", NULL, MAVLINK_TYPE_UINT8_T, 0, 25, offsetof(mavlink_cc_event_t, subsystem) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 26, offsetof(mavlink_cc_event_t, schema_version) }, \
         } \
}
#endif

/**
 * @brief Pack a cc_event message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param event_id  Event identifier (PX4 events namespace or CC-defined block).
 * @param argument0  Event argument 0 (meaning defined per event_id).
 * @param argument1  Event argument 1 (meaning defined per event_id).
 * @param severity  Event severity.
 * @param subsystem  Related subsystem.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_event_pack(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t sequence, uint32_t event_id, uint32_t argument0, uint32_t argument1, uint8_t severity, uint8_t subsystem, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_EVENT_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, event_id);
    _mav_put_uint32_t(buf, 16, argument0);
    _mav_put_uint32_t(buf, 20, argument1);
    _mav_put_uint8_t(buf, 24, severity);
    _mav_put_uint8_t(buf, 25, subsystem);
    _mav_put_uint8_t(buf, 26, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_EVENT_LEN);
#else
    mavlink_cc_event_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.event_id = event_id;
    packet.argument0 = argument0;
    packet.argument1 = argument1;
    packet.severity = severity;
    packet.subsystem = subsystem;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_EVENT_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_EVENT;
    return mavlink_finalize_message(msg, system_id, component_id, MAVLINK_MSG_ID_CC_EVENT_MIN_LEN, MAVLINK_MSG_ID_CC_EVENT_LEN, MAVLINK_MSG_ID_CC_EVENT_CRC);
}

/**
 * @brief Pack a cc_event message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param event_id  Event identifier (PX4 events namespace or CC-defined block).
 * @param argument0  Event argument 0 (meaning defined per event_id).
 * @param argument1  Event argument 1 (meaning defined per event_id).
 * @param severity  Event severity.
 * @param subsystem  Related subsystem.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_event_pack_status(uint8_t system_id, uint8_t component_id, mavlink_status_t *_status, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t sequence, uint32_t event_id, uint32_t argument0, uint32_t argument1, uint8_t severity, uint8_t subsystem, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_EVENT_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, event_id);
    _mav_put_uint32_t(buf, 16, argument0);
    _mav_put_uint32_t(buf, 20, argument1);
    _mav_put_uint8_t(buf, 24, severity);
    _mav_put_uint8_t(buf, 25, subsystem);
    _mav_put_uint8_t(buf, 26, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_EVENT_LEN);
#else
    mavlink_cc_event_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.event_id = event_id;
    packet.argument0 = argument0;
    packet.argument1 = argument1;
    packet.severity = severity;
    packet.subsystem = subsystem;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_EVENT_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_EVENT;
#if MAVLINK_CRC_EXTRA
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_EVENT_MIN_LEN, MAVLINK_MSG_ID_CC_EVENT_LEN, MAVLINK_MSG_ID_CC_EVENT_CRC);
#else
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_EVENT_MIN_LEN, MAVLINK_MSG_ID_CC_EVENT_LEN);
#endif
}

/**
 * @brief Pack a cc_event message on a channel
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param event_id  Event identifier (PX4 events namespace or CC-defined block).
 * @param argument0  Event argument 0 (meaning defined per event_id).
 * @param argument1  Event argument 1 (meaning defined per event_id).
 * @param severity  Event severity.
 * @param subsystem  Related subsystem.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_event_pack_chan(uint8_t system_id, uint8_t component_id, uint8_t chan,
                               mavlink_message_t* msg,
                                   uint64_t fc_timestamp_us,uint32_t sequence,uint32_t event_id,uint32_t argument0,uint32_t argument1,uint8_t severity,uint8_t subsystem,uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_EVENT_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, event_id);
    _mav_put_uint32_t(buf, 16, argument0);
    _mav_put_uint32_t(buf, 20, argument1);
    _mav_put_uint8_t(buf, 24, severity);
    _mav_put_uint8_t(buf, 25, subsystem);
    _mav_put_uint8_t(buf, 26, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_EVENT_LEN);
#else
    mavlink_cc_event_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.event_id = event_id;
    packet.argument0 = argument0;
    packet.argument1 = argument1;
    packet.severity = severity;
    packet.subsystem = subsystem;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_EVENT_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_EVENT;
    return mavlink_finalize_message_chan(msg, system_id, component_id, chan, MAVLINK_MSG_ID_CC_EVENT_MIN_LEN, MAVLINK_MSG_ID_CC_EVENT_LEN, MAVLINK_MSG_ID_CC_EVENT_CRC);
}

/**
 * @brief Encode a cc_event struct
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 * @param cc_event C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_event_encode(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg, const mavlink_cc_event_t* cc_event)
{
    return mavlink_msg_cc_event_pack(system_id, component_id, msg, cc_event->fc_timestamp_us, cc_event->sequence, cc_event->event_id, cc_event->argument0, cc_event->argument1, cc_event->severity, cc_event->subsystem, cc_event->schema_version);
}

/**
 * @brief Encode a cc_event struct on a channel
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param cc_event C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_event_encode_chan(uint8_t system_id, uint8_t component_id, uint8_t chan, mavlink_message_t* msg, const mavlink_cc_event_t* cc_event)
{
    return mavlink_msg_cc_event_pack_chan(system_id, component_id, chan, msg, cc_event->fc_timestamp_us, cc_event->sequence, cc_event->event_id, cc_event->argument0, cc_event->argument1, cc_event->severity, cc_event->subsystem, cc_event->schema_version);
}

/**
 * @brief Encode a cc_event struct with provided status structure
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 * @param cc_event C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_event_encode_status(uint8_t system_id, uint8_t component_id, mavlink_status_t* _status, mavlink_message_t* msg, const mavlink_cc_event_t* cc_event)
{
    return mavlink_msg_cc_event_pack_status(system_id, component_id, _status, msg,  cc_event->fc_timestamp_us, cc_event->sequence, cc_event->event_id, cc_event->argument0, cc_event->argument1, cc_event->severity, cc_event->subsystem, cc_event->schema_version);
}

/**
 * @brief Send a cc_event message
 * @param chan MAVLink channel to send the message
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter.
 * @param event_id  Event identifier (PX4 events namespace or CC-defined block).
 * @param argument0  Event argument 0 (meaning defined per event_id).
 * @param argument1  Event argument 1 (meaning defined per event_id).
 * @param severity  Event severity.
 * @param subsystem  Related subsystem.
 * @param schema_version  Payload schema version.
 */
#ifdef MAVLINK_USE_CONVENIENCE_FUNCTIONS

static inline void mavlink_msg_cc_event_send(mavlink_channel_t chan, uint64_t fc_timestamp_us, uint32_t sequence, uint32_t event_id, uint32_t argument0, uint32_t argument1, uint8_t severity, uint8_t subsystem, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_EVENT_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, event_id);
    _mav_put_uint32_t(buf, 16, argument0);
    _mav_put_uint32_t(buf, 20, argument1);
    _mav_put_uint8_t(buf, 24, severity);
    _mav_put_uint8_t(buf, 25, subsystem);
    _mav_put_uint8_t(buf, 26, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_EVENT, buf, MAVLINK_MSG_ID_CC_EVENT_MIN_LEN, MAVLINK_MSG_ID_CC_EVENT_LEN, MAVLINK_MSG_ID_CC_EVENT_CRC);
#else
    mavlink_cc_event_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.event_id = event_id;
    packet.argument0 = argument0;
    packet.argument1 = argument1;
    packet.severity = severity;
    packet.subsystem = subsystem;
    packet.schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_EVENT, (const char *)&packet, MAVLINK_MSG_ID_CC_EVENT_MIN_LEN, MAVLINK_MSG_ID_CC_EVENT_LEN, MAVLINK_MSG_ID_CC_EVENT_CRC);
#endif
}

/**
 * @brief Send a cc_event message
 * @param chan MAVLink channel to send the message
 * @param struct The MAVLink struct to serialize
 */
static inline void mavlink_msg_cc_event_send_struct(mavlink_channel_t chan, const mavlink_cc_event_t* cc_event)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    mavlink_msg_cc_event_send(chan, cc_event->fc_timestamp_us, cc_event->sequence, cc_event->event_id, cc_event->argument0, cc_event->argument1, cc_event->severity, cc_event->subsystem, cc_event->schema_version);
#else
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_EVENT, (const char *)cc_event, MAVLINK_MSG_ID_CC_EVENT_MIN_LEN, MAVLINK_MSG_ID_CC_EVENT_LEN, MAVLINK_MSG_ID_CC_EVENT_CRC);
#endif
}

#if MAVLINK_MSG_ID_CC_EVENT_LEN <= MAVLINK_MAX_PAYLOAD_LEN
/*
  This variant of _send() can be used to save stack space by reusing
  memory from the receive buffer.  The caller provides a
  mavlink_message_t which is the size of a full mavlink message. This
  is usually the receive buffer for the channel, and allows a reply to an
  incoming message with minimum stack space usage.
 */
static inline void mavlink_msg_cc_event_send_buf(mavlink_message_t *msgbuf, mavlink_channel_t chan,  uint64_t fc_timestamp_us, uint32_t sequence, uint32_t event_id, uint32_t argument0, uint32_t argument1, uint8_t severity, uint8_t subsystem, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char *buf = (char *)msgbuf;
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, event_id);
    _mav_put_uint32_t(buf, 16, argument0);
    _mav_put_uint32_t(buf, 20, argument1);
    _mav_put_uint8_t(buf, 24, severity);
    _mav_put_uint8_t(buf, 25, subsystem);
    _mav_put_uint8_t(buf, 26, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_EVENT, buf, MAVLINK_MSG_ID_CC_EVENT_MIN_LEN, MAVLINK_MSG_ID_CC_EVENT_LEN, MAVLINK_MSG_ID_CC_EVENT_CRC);
#else
    mavlink_cc_event_t *packet = (mavlink_cc_event_t *)msgbuf;
    packet->fc_timestamp_us = fc_timestamp_us;
    packet->sequence = sequence;
    packet->event_id = event_id;
    packet->argument0 = argument0;
    packet->argument1 = argument1;
    packet->severity = severity;
    packet->subsystem = subsystem;
    packet->schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_EVENT, (const char *)packet, MAVLINK_MSG_ID_CC_EVENT_MIN_LEN, MAVLINK_MSG_ID_CC_EVENT_LEN, MAVLINK_MSG_ID_CC_EVENT_CRC);
#endif
}
#endif

#endif

// MESSAGE CC_EVENT UNPACKING


/**
 * @brief Get field fc_timestamp_us from cc_event message
 *
 * @return [us] FC monotonic time since PX4 boot.
 */
static inline uint64_t mavlink_msg_cc_event_get_fc_timestamp_us(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint64_t(msg,  0);
}

/**
 * @brief Get field sequence from cc_event message
 *
 * @return  Per-stream monotonic counter.
 */
static inline uint32_t mavlink_msg_cc_event_get_sequence(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  8);
}

/**
 * @brief Get field event_id from cc_event message
 *
 * @return  Event identifier (PX4 events namespace or CC-defined block).
 */
static inline uint32_t mavlink_msg_cc_event_get_event_id(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  12);
}

/**
 * @brief Get field argument0 from cc_event message
 *
 * @return  Event argument 0 (meaning defined per event_id).
 */
static inline uint32_t mavlink_msg_cc_event_get_argument0(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  16);
}

/**
 * @brief Get field argument1 from cc_event message
 *
 * @return  Event argument 1 (meaning defined per event_id).
 */
static inline uint32_t mavlink_msg_cc_event_get_argument1(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  20);
}

/**
 * @brief Get field severity from cc_event message
 *
 * @return  Event severity.
 */
static inline uint8_t mavlink_msg_cc_event_get_severity(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  24);
}

/**
 * @brief Get field subsystem from cc_event message
 *
 * @return  Related subsystem.
 */
static inline uint8_t mavlink_msg_cc_event_get_subsystem(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  25);
}

/**
 * @brief Get field schema_version from cc_event message
 *
 * @return  Payload schema version.
 */
static inline uint8_t mavlink_msg_cc_event_get_schema_version(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  26);
}

/**
 * @brief Decode a cc_event message into a struct
 *
 * @param msg The message to decode
 * @param cc_event C-struct to decode the message contents into
 */
static inline void mavlink_msg_cc_event_decode(const mavlink_message_t* msg, mavlink_cc_event_t* cc_event)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    cc_event->fc_timestamp_us = mavlink_msg_cc_event_get_fc_timestamp_us(msg);
    cc_event->sequence = mavlink_msg_cc_event_get_sequence(msg);
    cc_event->event_id = mavlink_msg_cc_event_get_event_id(msg);
    cc_event->argument0 = mavlink_msg_cc_event_get_argument0(msg);
    cc_event->argument1 = mavlink_msg_cc_event_get_argument1(msg);
    cc_event->severity = mavlink_msg_cc_event_get_severity(msg);
    cc_event->subsystem = mavlink_msg_cc_event_get_subsystem(msg);
    cc_event->schema_version = mavlink_msg_cc_event_get_schema_version(msg);
#else
        uint8_t len = msg->len < MAVLINK_MSG_ID_CC_EVENT_LEN? msg->len : MAVLINK_MSG_ID_CC_EVENT_LEN;
        memset(cc_event, 0, MAVLINK_MSG_ID_CC_EVENT_LEN);
    memcpy(cc_event, _MAV_PAYLOAD(msg), len);
#endif
}
