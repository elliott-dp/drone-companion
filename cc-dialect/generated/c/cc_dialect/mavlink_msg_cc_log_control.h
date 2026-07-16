#pragma once
// MESSAGE CC_LOG_CONTROL PACKING

#define MAVLINK_MSG_ID_CC_LOG_CONTROL 54013


typedef struct __mavlink_cc_log_control_t {
 uint64_t companion_timestamp_us; /*< [us] CC monotonic time.*/
 uint32_t sequence; /*<  Monotonic per cc_boot_id.*/
 uint8_t requested_profile; /*<  Requested profile.*/
 uint8_t schema_version; /*<  Payload schema version.*/
} mavlink_cc_log_control_t;

#define MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN 14
#define MAVLINK_MSG_ID_CC_LOG_CONTROL_MIN_LEN 14
#define MAVLINK_MSG_ID_54013_LEN 14
#define MAVLINK_MSG_ID_54013_MIN_LEN 14

#define MAVLINK_MSG_ID_CC_LOG_CONTROL_CRC 64
#define MAVLINK_MSG_ID_54013_CRC 64



#if MAVLINK_COMMAND_24BIT
#define MAVLINK_MESSAGE_INFO_CC_LOG_CONTROL { \
    54013, \
    "CC_LOG_CONTROL", \
    4, \
    {  { "companion_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_log_control_t, companion_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_log_control_t, sequence) }, \
         { "requested_profile", NULL, MAVLINK_TYPE_UINT8_T, 0, 12, offsetof(mavlink_cc_log_control_t, requested_profile) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 13, offsetof(mavlink_cc_log_control_t, schema_version) }, \
         } \
}
#else
#define MAVLINK_MESSAGE_INFO_CC_LOG_CONTROL { \
    "CC_LOG_CONTROL", \
    4, \
    {  { "companion_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_log_control_t, companion_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_log_control_t, sequence) }, \
         { "requested_profile", NULL, MAVLINK_TYPE_UINT8_T, 0, 12, offsetof(mavlink_cc_log_control_t, requested_profile) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 13, offsetof(mavlink_cc_log_control_t, schema_version) }, \
         } \
}
#endif

/**
 * @brief Pack a cc_log_control message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 *
 * @param companion_timestamp_us [us] CC monotonic time.
 * @param sequence  Monotonic per cc_boot_id.
 * @param requested_profile  Requested profile.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_log_control_pack(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg,
                               uint64_t companion_timestamp_us, uint32_t sequence, uint8_t requested_profile, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN];
    _mav_put_uint64_t(buf, 0, companion_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint8_t(buf, 12, requested_profile);
    _mav_put_uint8_t(buf, 13, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN);
#else
    mavlink_cc_log_control_t packet;
    packet.companion_timestamp_us = companion_timestamp_us;
    packet.sequence = sequence;
    packet.requested_profile = requested_profile;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_LOG_CONTROL;
    return mavlink_finalize_message(msg, system_id, component_id, MAVLINK_MSG_ID_CC_LOG_CONTROL_MIN_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_CRC);
}

/**
 * @brief Pack a cc_log_control message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 *
 * @param companion_timestamp_us [us] CC monotonic time.
 * @param sequence  Monotonic per cc_boot_id.
 * @param requested_profile  Requested profile.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_log_control_pack_status(uint8_t system_id, uint8_t component_id, mavlink_status_t *_status, mavlink_message_t* msg,
                               uint64_t companion_timestamp_us, uint32_t sequence, uint8_t requested_profile, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN];
    _mav_put_uint64_t(buf, 0, companion_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint8_t(buf, 12, requested_profile);
    _mav_put_uint8_t(buf, 13, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN);
#else
    mavlink_cc_log_control_t packet;
    packet.companion_timestamp_us = companion_timestamp_us;
    packet.sequence = sequence;
    packet.requested_profile = requested_profile;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_LOG_CONTROL;
#if MAVLINK_CRC_EXTRA
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_LOG_CONTROL_MIN_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_CRC);
#else
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_LOG_CONTROL_MIN_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN);
#endif
}

/**
 * @brief Pack a cc_log_control message on a channel
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param companion_timestamp_us [us] CC monotonic time.
 * @param sequence  Monotonic per cc_boot_id.
 * @param requested_profile  Requested profile.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_log_control_pack_chan(uint8_t system_id, uint8_t component_id, uint8_t chan,
                               mavlink_message_t* msg,
                                   uint64_t companion_timestamp_us,uint32_t sequence,uint8_t requested_profile,uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN];
    _mav_put_uint64_t(buf, 0, companion_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint8_t(buf, 12, requested_profile);
    _mav_put_uint8_t(buf, 13, schema_version);

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN);
#else
    mavlink_cc_log_control_t packet;
    packet.companion_timestamp_us = companion_timestamp_us;
    packet.sequence = sequence;
    packet.requested_profile = requested_profile;
    packet.schema_version = schema_version;

        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_LOG_CONTROL;
    return mavlink_finalize_message_chan(msg, system_id, component_id, chan, MAVLINK_MSG_ID_CC_LOG_CONTROL_MIN_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_CRC);
}

/**
 * @brief Encode a cc_log_control struct
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 * @param cc_log_control C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_log_control_encode(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg, const mavlink_cc_log_control_t* cc_log_control)
{
    return mavlink_msg_cc_log_control_pack(system_id, component_id, msg, cc_log_control->companion_timestamp_us, cc_log_control->sequence, cc_log_control->requested_profile, cc_log_control->schema_version);
}

/**
 * @brief Encode a cc_log_control struct on a channel
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param cc_log_control C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_log_control_encode_chan(uint8_t system_id, uint8_t component_id, uint8_t chan, mavlink_message_t* msg, const mavlink_cc_log_control_t* cc_log_control)
{
    return mavlink_msg_cc_log_control_pack_chan(system_id, component_id, chan, msg, cc_log_control->companion_timestamp_us, cc_log_control->sequence, cc_log_control->requested_profile, cc_log_control->schema_version);
}

/**
 * @brief Encode a cc_log_control struct with provided status structure
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 * @param cc_log_control C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_log_control_encode_status(uint8_t system_id, uint8_t component_id, mavlink_status_t* _status, mavlink_message_t* msg, const mavlink_cc_log_control_t* cc_log_control)
{
    return mavlink_msg_cc_log_control_pack_status(system_id, component_id, _status, msg,  cc_log_control->companion_timestamp_us, cc_log_control->sequence, cc_log_control->requested_profile, cc_log_control->schema_version);
}

/**
 * @brief Send a cc_log_control message
 * @param chan MAVLink channel to send the message
 *
 * @param companion_timestamp_us [us] CC monotonic time.
 * @param sequence  Monotonic per cc_boot_id.
 * @param requested_profile  Requested profile.
 * @param schema_version  Payload schema version.
 */
#ifdef MAVLINK_USE_CONVENIENCE_FUNCTIONS

static inline void mavlink_msg_cc_log_control_send(mavlink_channel_t chan, uint64_t companion_timestamp_us, uint32_t sequence, uint8_t requested_profile, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN];
    _mav_put_uint64_t(buf, 0, companion_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint8_t(buf, 12, requested_profile);
    _mav_put_uint8_t(buf, 13, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_LOG_CONTROL, buf, MAVLINK_MSG_ID_CC_LOG_CONTROL_MIN_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_CRC);
#else
    mavlink_cc_log_control_t packet;
    packet.companion_timestamp_us = companion_timestamp_us;
    packet.sequence = sequence;
    packet.requested_profile = requested_profile;
    packet.schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_LOG_CONTROL, (const char *)&packet, MAVLINK_MSG_ID_CC_LOG_CONTROL_MIN_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_CRC);
#endif
}

/**
 * @brief Send a cc_log_control message
 * @param chan MAVLink channel to send the message
 * @param struct The MAVLink struct to serialize
 */
static inline void mavlink_msg_cc_log_control_send_struct(mavlink_channel_t chan, const mavlink_cc_log_control_t* cc_log_control)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    mavlink_msg_cc_log_control_send(chan, cc_log_control->companion_timestamp_us, cc_log_control->sequence, cc_log_control->requested_profile, cc_log_control->schema_version);
#else
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_LOG_CONTROL, (const char *)cc_log_control, MAVLINK_MSG_ID_CC_LOG_CONTROL_MIN_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_CRC);
#endif
}

#if MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN <= MAVLINK_MAX_PAYLOAD_LEN
/*
  This variant of _send() can be used to save stack space by reusing
  memory from the receive buffer.  The caller provides a
  mavlink_message_t which is the size of a full mavlink message. This
  is usually the receive buffer for the channel, and allows a reply to an
  incoming message with minimum stack space usage.
 */
static inline void mavlink_msg_cc_log_control_send_buf(mavlink_message_t *msgbuf, mavlink_channel_t chan,  uint64_t companion_timestamp_us, uint32_t sequence, uint8_t requested_profile, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char *buf = (char *)msgbuf;
    _mav_put_uint64_t(buf, 0, companion_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint8_t(buf, 12, requested_profile);
    _mav_put_uint8_t(buf, 13, schema_version);

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_LOG_CONTROL, buf, MAVLINK_MSG_ID_CC_LOG_CONTROL_MIN_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_CRC);
#else
    mavlink_cc_log_control_t *packet = (mavlink_cc_log_control_t *)msgbuf;
    packet->companion_timestamp_us = companion_timestamp_us;
    packet->sequence = sequence;
    packet->requested_profile = requested_profile;
    packet->schema_version = schema_version;

    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_LOG_CONTROL, (const char *)packet, MAVLINK_MSG_ID_CC_LOG_CONTROL_MIN_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN, MAVLINK_MSG_ID_CC_LOG_CONTROL_CRC);
#endif
}
#endif

#endif

// MESSAGE CC_LOG_CONTROL UNPACKING


/**
 * @brief Get field companion_timestamp_us from cc_log_control message
 *
 * @return [us] CC monotonic time.
 */
static inline uint64_t mavlink_msg_cc_log_control_get_companion_timestamp_us(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint64_t(msg,  0);
}

/**
 * @brief Get field sequence from cc_log_control message
 *
 * @return  Monotonic per cc_boot_id.
 */
static inline uint32_t mavlink_msg_cc_log_control_get_sequence(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  8);
}

/**
 * @brief Get field requested_profile from cc_log_control message
 *
 * @return  Requested profile.
 */
static inline uint8_t mavlink_msg_cc_log_control_get_requested_profile(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  12);
}

/**
 * @brief Get field schema_version from cc_log_control message
 *
 * @return  Payload schema version.
 */
static inline uint8_t mavlink_msg_cc_log_control_get_schema_version(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  13);
}

/**
 * @brief Decode a cc_log_control message into a struct
 *
 * @param msg The message to decode
 * @param cc_log_control C-struct to decode the message contents into
 */
static inline void mavlink_msg_cc_log_control_decode(const mavlink_message_t* msg, mavlink_cc_log_control_t* cc_log_control)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    cc_log_control->companion_timestamp_us = mavlink_msg_cc_log_control_get_companion_timestamp_us(msg);
    cc_log_control->sequence = mavlink_msg_cc_log_control_get_sequence(msg);
    cc_log_control->requested_profile = mavlink_msg_cc_log_control_get_requested_profile(msg);
    cc_log_control->schema_version = mavlink_msg_cc_log_control_get_schema_version(msg);
#else
        uint8_t len = msg->len < MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN? msg->len : MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN;
        memset(cc_log_control, 0, MAVLINK_MSG_ID_CC_LOG_CONTROL_LEN);
    memcpy(cc_log_control, _MAV_PAYLOAD(msg), len);
#endif
}
