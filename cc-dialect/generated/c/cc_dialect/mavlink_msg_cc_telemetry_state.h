#pragma once
// MESSAGE CC_TELEMETRY_STATE PACKING

#define MAVLINK_MSG_ID_CC_TELEMETRY_STATE 54000


typedef struct __mavlink_cc_telemetry_state_t {
 uint64_t fc_timestamp_us; /*< [us] FC monotonic time since PX4 boot.*/
 uint32_t sequence; /*<  Per-stream monotonic counter (wraps 2^32).*/
 uint32_t px4_boot_id; /*<  FC boot identity (constant per boot).*/
 uint32_t mission_id; /*<  Mission identity echoed from CC_MISSION_CONTEXT (0 if none).*/
 uint32_t failsafe_flags; /*<  PX4 failsafe flags bitfield (as in failsafe_flags uORB).*/
 float q[4]; /*<  Attitude quaternion (w, x, y, z), body to NED.*/
 float angular_velocity[3]; /*< [rad/s] Body angular rates (x, y, z).*/
 float position_ned[3]; /*< [m] Local position NED.*/
 float velocity_ned[3]; /*< [m/s] Local velocity NED.*/
 float heading; /*< [rad] Heading (yaw), NED, [-pi, pi].*/
 uint8_t nav_state; /*<  PX4 nav_state (vehicle_status).*/
 uint8_t arming_state; /*<  PX4 arming_state (vehicle_status).*/
 uint8_t vehicle_type; /*<  PX4 vehicle type.*/
 uint8_t estimator_valid; /*<  1 if local position/attitude estimates valid.*/
 uint8_t control_mode_flags; /*<  Compact vehicle_control_mode bitfield.*/
 uint8_t schema_version; /*<  Payload schema version.*/
} mavlink_cc_telemetry_state_t;

#define MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN 86
#define MAVLINK_MSG_ID_CC_TELEMETRY_STATE_MIN_LEN 86
#define MAVLINK_MSG_ID_54000_LEN 86
#define MAVLINK_MSG_ID_54000_MIN_LEN 86

#define MAVLINK_MSG_ID_CC_TELEMETRY_STATE_CRC 139
#define MAVLINK_MSG_ID_54000_CRC 139

#define MAVLINK_MSG_CC_TELEMETRY_STATE_FIELD_Q_LEN 4
#define MAVLINK_MSG_CC_TELEMETRY_STATE_FIELD_ANGULAR_VELOCITY_LEN 3
#define MAVLINK_MSG_CC_TELEMETRY_STATE_FIELD_POSITION_NED_LEN 3
#define MAVLINK_MSG_CC_TELEMETRY_STATE_FIELD_VELOCITY_NED_LEN 3

#if MAVLINK_COMMAND_24BIT
#define MAVLINK_MESSAGE_INFO_CC_TELEMETRY_STATE { \
    54000, \
    "CC_TELEMETRY_STATE", \
    16, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_telemetry_state_t, fc_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_telemetry_state_t, sequence) }, \
         { "px4_boot_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 12, offsetof(mavlink_cc_telemetry_state_t, px4_boot_id) }, \
         { "mission_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 16, offsetof(mavlink_cc_telemetry_state_t, mission_id) }, \
         { "failsafe_flags", NULL, MAVLINK_TYPE_UINT32_T, 0, 20, offsetof(mavlink_cc_telemetry_state_t, failsafe_flags) }, \
         { "q", NULL, MAVLINK_TYPE_FLOAT, 4, 24, offsetof(mavlink_cc_telemetry_state_t, q) }, \
         { "angular_velocity", NULL, MAVLINK_TYPE_FLOAT, 3, 40, offsetof(mavlink_cc_telemetry_state_t, angular_velocity) }, \
         { "position_ned", NULL, MAVLINK_TYPE_FLOAT, 3, 52, offsetof(mavlink_cc_telemetry_state_t, position_ned) }, \
         { "velocity_ned", NULL, MAVLINK_TYPE_FLOAT, 3, 64, offsetof(mavlink_cc_telemetry_state_t, velocity_ned) }, \
         { "heading", NULL, MAVLINK_TYPE_FLOAT, 0, 76, offsetof(mavlink_cc_telemetry_state_t, heading) }, \
         { "nav_state", NULL, MAVLINK_TYPE_UINT8_T, 0, 80, offsetof(mavlink_cc_telemetry_state_t, nav_state) }, \
         { "arming_state", NULL, MAVLINK_TYPE_UINT8_T, 0, 81, offsetof(mavlink_cc_telemetry_state_t, arming_state) }, \
         { "vehicle_type", NULL, MAVLINK_TYPE_UINT8_T, 0, 82, offsetof(mavlink_cc_telemetry_state_t, vehicle_type) }, \
         { "estimator_valid", NULL, MAVLINK_TYPE_UINT8_T, 0, 83, offsetof(mavlink_cc_telemetry_state_t, estimator_valid) }, \
         { "control_mode_flags", NULL, MAVLINK_TYPE_UINT8_T, 0, 84, offsetof(mavlink_cc_telemetry_state_t, control_mode_flags) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 85, offsetof(mavlink_cc_telemetry_state_t, schema_version) }, \
         } \
}
#else
#define MAVLINK_MESSAGE_INFO_CC_TELEMETRY_STATE { \
    "CC_TELEMETRY_STATE", \
    16, \
    {  { "fc_timestamp_us", NULL, MAVLINK_TYPE_UINT64_T, 0, 0, offsetof(mavlink_cc_telemetry_state_t, fc_timestamp_us) }, \
         { "sequence", NULL, MAVLINK_TYPE_UINT32_T, 0, 8, offsetof(mavlink_cc_telemetry_state_t, sequence) }, \
         { "px4_boot_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 12, offsetof(mavlink_cc_telemetry_state_t, px4_boot_id) }, \
         { "mission_id", NULL, MAVLINK_TYPE_UINT32_T, 0, 16, offsetof(mavlink_cc_telemetry_state_t, mission_id) }, \
         { "failsafe_flags", NULL, MAVLINK_TYPE_UINT32_T, 0, 20, offsetof(mavlink_cc_telemetry_state_t, failsafe_flags) }, \
         { "q", NULL, MAVLINK_TYPE_FLOAT, 4, 24, offsetof(mavlink_cc_telemetry_state_t, q) }, \
         { "angular_velocity", NULL, MAVLINK_TYPE_FLOAT, 3, 40, offsetof(mavlink_cc_telemetry_state_t, angular_velocity) }, \
         { "position_ned", NULL, MAVLINK_TYPE_FLOAT, 3, 52, offsetof(mavlink_cc_telemetry_state_t, position_ned) }, \
         { "velocity_ned", NULL, MAVLINK_TYPE_FLOAT, 3, 64, offsetof(mavlink_cc_telemetry_state_t, velocity_ned) }, \
         { "heading", NULL, MAVLINK_TYPE_FLOAT, 0, 76, offsetof(mavlink_cc_telemetry_state_t, heading) }, \
         { "nav_state", NULL, MAVLINK_TYPE_UINT8_T, 0, 80, offsetof(mavlink_cc_telemetry_state_t, nav_state) }, \
         { "arming_state", NULL, MAVLINK_TYPE_UINT8_T, 0, 81, offsetof(mavlink_cc_telemetry_state_t, arming_state) }, \
         { "vehicle_type", NULL, MAVLINK_TYPE_UINT8_T, 0, 82, offsetof(mavlink_cc_telemetry_state_t, vehicle_type) }, \
         { "estimator_valid", NULL, MAVLINK_TYPE_UINT8_T, 0, 83, offsetof(mavlink_cc_telemetry_state_t, estimator_valid) }, \
         { "control_mode_flags", NULL, MAVLINK_TYPE_UINT8_T, 0, 84, offsetof(mavlink_cc_telemetry_state_t, control_mode_flags) }, \
         { "schema_version", NULL, MAVLINK_TYPE_UINT8_T, 0, 85, offsetof(mavlink_cc_telemetry_state_t, schema_version) }, \
         } \
}
#endif

/**
 * @brief Pack a cc_telemetry_state message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter (wraps 2^32).
 * @param px4_boot_id  FC boot identity (constant per boot).
 * @param mission_id  Mission identity echoed from CC_MISSION_CONTEXT (0 if none).
 * @param failsafe_flags  PX4 failsafe flags bitfield (as in failsafe_flags uORB).
 * @param q  Attitude quaternion (w, x, y, z), body to NED.
 * @param angular_velocity [rad/s] Body angular rates (x, y, z).
 * @param position_ned [m] Local position NED.
 * @param velocity_ned [m/s] Local velocity NED.
 * @param heading [rad] Heading (yaw), NED, [-pi, pi].
 * @param nav_state  PX4 nav_state (vehicle_status).
 * @param arming_state  PX4 arming_state (vehicle_status).
 * @param vehicle_type  PX4 vehicle type.
 * @param estimator_valid  1 if local position/attitude estimates valid.
 * @param control_mode_flags  Compact vehicle_control_mode bitfield.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_state_pack(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t sequence, uint32_t px4_boot_id, uint32_t mission_id, uint32_t failsafe_flags, const float *q, const float *angular_velocity, const float *position_ned, const float *velocity_ned, float heading, uint8_t nav_state, uint8_t arming_state, uint8_t vehicle_type, uint8_t estimator_valid, uint8_t control_mode_flags, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, px4_boot_id);
    _mav_put_uint32_t(buf, 16, mission_id);
    _mav_put_uint32_t(buf, 20, failsafe_flags);
    _mav_put_float(buf, 76, heading);
    _mav_put_uint8_t(buf, 80, nav_state);
    _mav_put_uint8_t(buf, 81, arming_state);
    _mav_put_uint8_t(buf, 82, vehicle_type);
    _mav_put_uint8_t(buf, 83, estimator_valid);
    _mav_put_uint8_t(buf, 84, control_mode_flags);
    _mav_put_uint8_t(buf, 85, schema_version);
    _mav_put_float_array(buf, 24, q, 4);
    _mav_put_float_array(buf, 40, angular_velocity, 3);
    _mav_put_float_array(buf, 52, position_ned, 3);
    _mav_put_float_array(buf, 64, velocity_ned, 3);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN);
#else
    mavlink_cc_telemetry_state_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.px4_boot_id = px4_boot_id;
    packet.mission_id = mission_id;
    packet.failsafe_flags = failsafe_flags;
    packet.heading = heading;
    packet.nav_state = nav_state;
    packet.arming_state = arming_state;
    packet.vehicle_type = vehicle_type;
    packet.estimator_valid = estimator_valid;
    packet.control_mode_flags = control_mode_flags;
    packet.schema_version = schema_version;
    mav_array_assign_float(packet.q, q, 4);
    mav_array_assign_float(packet.angular_velocity, angular_velocity, 3);
    mav_array_assign_float(packet.position_ned, position_ned, 3);
    mav_array_assign_float(packet.velocity_ned, velocity_ned, 3);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_STATE;
    return mavlink_finalize_message(msg, system_id, component_id, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_CRC);
}

/**
 * @brief Pack a cc_telemetry_state message
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter (wraps 2^32).
 * @param px4_boot_id  FC boot identity (constant per boot).
 * @param mission_id  Mission identity echoed from CC_MISSION_CONTEXT (0 if none).
 * @param failsafe_flags  PX4 failsafe flags bitfield (as in failsafe_flags uORB).
 * @param q  Attitude quaternion (w, x, y, z), body to NED.
 * @param angular_velocity [rad/s] Body angular rates (x, y, z).
 * @param position_ned [m] Local position NED.
 * @param velocity_ned [m/s] Local velocity NED.
 * @param heading [rad] Heading (yaw), NED, [-pi, pi].
 * @param nav_state  PX4 nav_state (vehicle_status).
 * @param arming_state  PX4 arming_state (vehicle_status).
 * @param vehicle_type  PX4 vehicle type.
 * @param estimator_valid  1 if local position/attitude estimates valid.
 * @param control_mode_flags  Compact vehicle_control_mode bitfield.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_state_pack_status(uint8_t system_id, uint8_t component_id, mavlink_status_t *_status, mavlink_message_t* msg,
                               uint64_t fc_timestamp_us, uint32_t sequence, uint32_t px4_boot_id, uint32_t mission_id, uint32_t failsafe_flags, const float *q, const float *angular_velocity, const float *position_ned, const float *velocity_ned, float heading, uint8_t nav_state, uint8_t arming_state, uint8_t vehicle_type, uint8_t estimator_valid, uint8_t control_mode_flags, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, px4_boot_id);
    _mav_put_uint32_t(buf, 16, mission_id);
    _mav_put_uint32_t(buf, 20, failsafe_flags);
    _mav_put_float(buf, 76, heading);
    _mav_put_uint8_t(buf, 80, nav_state);
    _mav_put_uint8_t(buf, 81, arming_state);
    _mav_put_uint8_t(buf, 82, vehicle_type);
    _mav_put_uint8_t(buf, 83, estimator_valid);
    _mav_put_uint8_t(buf, 84, control_mode_flags);
    _mav_put_uint8_t(buf, 85, schema_version);
    _mav_put_float_array(buf, 24, q, 4);
    _mav_put_float_array(buf, 40, angular_velocity, 3);
    _mav_put_float_array(buf, 52, position_ned, 3);
    _mav_put_float_array(buf, 64, velocity_ned, 3);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN);
#else
    mavlink_cc_telemetry_state_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.px4_boot_id = px4_boot_id;
    packet.mission_id = mission_id;
    packet.failsafe_flags = failsafe_flags;
    packet.heading = heading;
    packet.nav_state = nav_state;
    packet.arming_state = arming_state;
    packet.vehicle_type = vehicle_type;
    packet.estimator_valid = estimator_valid;
    packet.control_mode_flags = control_mode_flags;
    packet.schema_version = schema_version;
    mav_array_memcpy(packet.q, q, sizeof(float)*4);
    mav_array_memcpy(packet.angular_velocity, angular_velocity, sizeof(float)*3);
    mav_array_memcpy(packet.position_ned, position_ned, sizeof(float)*3);
    mav_array_memcpy(packet.velocity_ned, velocity_ned, sizeof(float)*3);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_STATE;
#if MAVLINK_CRC_EXTRA
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_CRC);
#else
    return mavlink_finalize_message_buffer(msg, system_id, component_id, _status, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN);
#endif
}

/**
 * @brief Pack a cc_telemetry_state message on a channel
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter (wraps 2^32).
 * @param px4_boot_id  FC boot identity (constant per boot).
 * @param mission_id  Mission identity echoed from CC_MISSION_CONTEXT (0 if none).
 * @param failsafe_flags  PX4 failsafe flags bitfield (as in failsafe_flags uORB).
 * @param q  Attitude quaternion (w, x, y, z), body to NED.
 * @param angular_velocity [rad/s] Body angular rates (x, y, z).
 * @param position_ned [m] Local position NED.
 * @param velocity_ned [m/s] Local velocity NED.
 * @param heading [rad] Heading (yaw), NED, [-pi, pi].
 * @param nav_state  PX4 nav_state (vehicle_status).
 * @param arming_state  PX4 arming_state (vehicle_status).
 * @param vehicle_type  PX4 vehicle type.
 * @param estimator_valid  1 if local position/attitude estimates valid.
 * @param control_mode_flags  Compact vehicle_control_mode bitfield.
 * @param schema_version  Payload schema version.
 * @return length of the message in bytes (excluding serial stream start sign)
 */
static inline uint16_t mavlink_msg_cc_telemetry_state_pack_chan(uint8_t system_id, uint8_t component_id, uint8_t chan,
                               mavlink_message_t* msg,
                                   uint64_t fc_timestamp_us,uint32_t sequence,uint32_t px4_boot_id,uint32_t mission_id,uint32_t failsafe_flags,const float *q,const float *angular_velocity,const float *position_ned,const float *velocity_ned,float heading,uint8_t nav_state,uint8_t arming_state,uint8_t vehicle_type,uint8_t estimator_valid,uint8_t control_mode_flags,uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, px4_boot_id);
    _mav_put_uint32_t(buf, 16, mission_id);
    _mav_put_uint32_t(buf, 20, failsafe_flags);
    _mav_put_float(buf, 76, heading);
    _mav_put_uint8_t(buf, 80, nav_state);
    _mav_put_uint8_t(buf, 81, arming_state);
    _mav_put_uint8_t(buf, 82, vehicle_type);
    _mav_put_uint8_t(buf, 83, estimator_valid);
    _mav_put_uint8_t(buf, 84, control_mode_flags);
    _mav_put_uint8_t(buf, 85, schema_version);
    _mav_put_float_array(buf, 24, q, 4);
    _mav_put_float_array(buf, 40, angular_velocity, 3);
    _mav_put_float_array(buf, 52, position_ned, 3);
    _mav_put_float_array(buf, 64, velocity_ned, 3);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), buf, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN);
#else
    mavlink_cc_telemetry_state_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.px4_boot_id = px4_boot_id;
    packet.mission_id = mission_id;
    packet.failsafe_flags = failsafe_flags;
    packet.heading = heading;
    packet.nav_state = nav_state;
    packet.arming_state = arming_state;
    packet.vehicle_type = vehicle_type;
    packet.estimator_valid = estimator_valid;
    packet.control_mode_flags = control_mode_flags;
    packet.schema_version = schema_version;
    mav_array_assign_float(packet.q, q, 4);
    mav_array_assign_float(packet.angular_velocity, angular_velocity, 3);
    mav_array_assign_float(packet.position_ned, position_ned, 3);
    mav_array_assign_float(packet.velocity_ned, velocity_ned, 3);
        memcpy(_MAV_PAYLOAD_NON_CONST(msg), &packet, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN);
#endif

    msg->msgid = MAVLINK_MSG_ID_CC_TELEMETRY_STATE;
    return mavlink_finalize_message_chan(msg, system_id, component_id, chan, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_CRC);
}

/**
 * @brief Encode a cc_telemetry_state struct
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_state C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_state_encode(uint8_t system_id, uint8_t component_id, mavlink_message_t* msg, const mavlink_cc_telemetry_state_t* cc_telemetry_state)
{
    return mavlink_msg_cc_telemetry_state_pack(system_id, component_id, msg, cc_telemetry_state->fc_timestamp_us, cc_telemetry_state->sequence, cc_telemetry_state->px4_boot_id, cc_telemetry_state->mission_id, cc_telemetry_state->failsafe_flags, cc_telemetry_state->q, cc_telemetry_state->angular_velocity, cc_telemetry_state->position_ned, cc_telemetry_state->velocity_ned, cc_telemetry_state->heading, cc_telemetry_state->nav_state, cc_telemetry_state->arming_state, cc_telemetry_state->vehicle_type, cc_telemetry_state->estimator_valid, cc_telemetry_state->control_mode_flags, cc_telemetry_state->schema_version);
}

/**
 * @brief Encode a cc_telemetry_state struct on a channel
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param chan The MAVLink channel this message will be sent over
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_state C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_state_encode_chan(uint8_t system_id, uint8_t component_id, uint8_t chan, mavlink_message_t* msg, const mavlink_cc_telemetry_state_t* cc_telemetry_state)
{
    return mavlink_msg_cc_telemetry_state_pack_chan(system_id, component_id, chan, msg, cc_telemetry_state->fc_timestamp_us, cc_telemetry_state->sequence, cc_telemetry_state->px4_boot_id, cc_telemetry_state->mission_id, cc_telemetry_state->failsafe_flags, cc_telemetry_state->q, cc_telemetry_state->angular_velocity, cc_telemetry_state->position_ned, cc_telemetry_state->velocity_ned, cc_telemetry_state->heading, cc_telemetry_state->nav_state, cc_telemetry_state->arming_state, cc_telemetry_state->vehicle_type, cc_telemetry_state->estimator_valid, cc_telemetry_state->control_mode_flags, cc_telemetry_state->schema_version);
}

/**
 * @brief Encode a cc_telemetry_state struct with provided status structure
 *
 * @param system_id ID of this system
 * @param component_id ID of this component (e.g. 200 for IMU)
 * @param status MAVLink status structure
 * @param msg The MAVLink message to compress the data into
 * @param cc_telemetry_state C-struct to read the message contents from
 */
static inline uint16_t mavlink_msg_cc_telemetry_state_encode_status(uint8_t system_id, uint8_t component_id, mavlink_status_t* _status, mavlink_message_t* msg, const mavlink_cc_telemetry_state_t* cc_telemetry_state)
{
    return mavlink_msg_cc_telemetry_state_pack_status(system_id, component_id, _status, msg,  cc_telemetry_state->fc_timestamp_us, cc_telemetry_state->sequence, cc_telemetry_state->px4_boot_id, cc_telemetry_state->mission_id, cc_telemetry_state->failsafe_flags, cc_telemetry_state->q, cc_telemetry_state->angular_velocity, cc_telemetry_state->position_ned, cc_telemetry_state->velocity_ned, cc_telemetry_state->heading, cc_telemetry_state->nav_state, cc_telemetry_state->arming_state, cc_telemetry_state->vehicle_type, cc_telemetry_state->estimator_valid, cc_telemetry_state->control_mode_flags, cc_telemetry_state->schema_version);
}

/**
 * @brief Send a cc_telemetry_state message
 * @param chan MAVLink channel to send the message
 *
 * @param fc_timestamp_us [us] FC monotonic time since PX4 boot.
 * @param sequence  Per-stream monotonic counter (wraps 2^32).
 * @param px4_boot_id  FC boot identity (constant per boot).
 * @param mission_id  Mission identity echoed from CC_MISSION_CONTEXT (0 if none).
 * @param failsafe_flags  PX4 failsafe flags bitfield (as in failsafe_flags uORB).
 * @param q  Attitude quaternion (w, x, y, z), body to NED.
 * @param angular_velocity [rad/s] Body angular rates (x, y, z).
 * @param position_ned [m] Local position NED.
 * @param velocity_ned [m/s] Local velocity NED.
 * @param heading [rad] Heading (yaw), NED, [-pi, pi].
 * @param nav_state  PX4 nav_state (vehicle_status).
 * @param arming_state  PX4 arming_state (vehicle_status).
 * @param vehicle_type  PX4 vehicle type.
 * @param estimator_valid  1 if local position/attitude estimates valid.
 * @param control_mode_flags  Compact vehicle_control_mode bitfield.
 * @param schema_version  Payload schema version.
 */
#ifdef MAVLINK_USE_CONVENIENCE_FUNCTIONS

static inline void mavlink_msg_cc_telemetry_state_send(mavlink_channel_t chan, uint64_t fc_timestamp_us, uint32_t sequence, uint32_t px4_boot_id, uint32_t mission_id, uint32_t failsafe_flags, const float *q, const float *angular_velocity, const float *position_ned, const float *velocity_ned, float heading, uint8_t nav_state, uint8_t arming_state, uint8_t vehicle_type, uint8_t estimator_valid, uint8_t control_mode_flags, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char buf[MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN];
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, px4_boot_id);
    _mav_put_uint32_t(buf, 16, mission_id);
    _mav_put_uint32_t(buf, 20, failsafe_flags);
    _mav_put_float(buf, 76, heading);
    _mav_put_uint8_t(buf, 80, nav_state);
    _mav_put_uint8_t(buf, 81, arming_state);
    _mav_put_uint8_t(buf, 82, vehicle_type);
    _mav_put_uint8_t(buf, 83, estimator_valid);
    _mav_put_uint8_t(buf, 84, control_mode_flags);
    _mav_put_uint8_t(buf, 85, schema_version);
    _mav_put_float_array(buf, 24, q, 4);
    _mav_put_float_array(buf, 40, angular_velocity, 3);
    _mav_put_float_array(buf, 52, position_ned, 3);
    _mav_put_float_array(buf, 64, velocity_ned, 3);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_STATE, buf, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_CRC);
#else
    mavlink_cc_telemetry_state_t packet;
    packet.fc_timestamp_us = fc_timestamp_us;
    packet.sequence = sequence;
    packet.px4_boot_id = px4_boot_id;
    packet.mission_id = mission_id;
    packet.failsafe_flags = failsafe_flags;
    packet.heading = heading;
    packet.nav_state = nav_state;
    packet.arming_state = arming_state;
    packet.vehicle_type = vehicle_type;
    packet.estimator_valid = estimator_valid;
    packet.control_mode_flags = control_mode_flags;
    packet.schema_version = schema_version;
    mav_array_assign_float(packet.q, q, 4);
    mav_array_assign_float(packet.angular_velocity, angular_velocity, 3);
    mav_array_assign_float(packet.position_ned, position_ned, 3);
    mav_array_assign_float(packet.velocity_ned, velocity_ned, 3);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_STATE, (const char *)&packet, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_CRC);
#endif
}

/**
 * @brief Send a cc_telemetry_state message
 * @param chan MAVLink channel to send the message
 * @param struct The MAVLink struct to serialize
 */
static inline void mavlink_msg_cc_telemetry_state_send_struct(mavlink_channel_t chan, const mavlink_cc_telemetry_state_t* cc_telemetry_state)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    mavlink_msg_cc_telemetry_state_send(chan, cc_telemetry_state->fc_timestamp_us, cc_telemetry_state->sequence, cc_telemetry_state->px4_boot_id, cc_telemetry_state->mission_id, cc_telemetry_state->failsafe_flags, cc_telemetry_state->q, cc_telemetry_state->angular_velocity, cc_telemetry_state->position_ned, cc_telemetry_state->velocity_ned, cc_telemetry_state->heading, cc_telemetry_state->nav_state, cc_telemetry_state->arming_state, cc_telemetry_state->vehicle_type, cc_telemetry_state->estimator_valid, cc_telemetry_state->control_mode_flags, cc_telemetry_state->schema_version);
#else
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_STATE, (const char *)cc_telemetry_state, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_CRC);
#endif
}

#if MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN <= MAVLINK_MAX_PAYLOAD_LEN
/*
  This variant of _send() can be used to save stack space by reusing
  memory from the receive buffer.  The caller provides a
  mavlink_message_t which is the size of a full mavlink message. This
  is usually the receive buffer for the channel, and allows a reply to an
  incoming message with minimum stack space usage.
 */
static inline void mavlink_msg_cc_telemetry_state_send_buf(mavlink_message_t *msgbuf, mavlink_channel_t chan,  uint64_t fc_timestamp_us, uint32_t sequence, uint32_t px4_boot_id, uint32_t mission_id, uint32_t failsafe_flags, const float *q, const float *angular_velocity, const float *position_ned, const float *velocity_ned, float heading, uint8_t nav_state, uint8_t arming_state, uint8_t vehicle_type, uint8_t estimator_valid, uint8_t control_mode_flags, uint8_t schema_version)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    char *buf = (char *)msgbuf;
    _mav_put_uint64_t(buf, 0, fc_timestamp_us);
    _mav_put_uint32_t(buf, 8, sequence);
    _mav_put_uint32_t(buf, 12, px4_boot_id);
    _mav_put_uint32_t(buf, 16, mission_id);
    _mav_put_uint32_t(buf, 20, failsafe_flags);
    _mav_put_float(buf, 76, heading);
    _mav_put_uint8_t(buf, 80, nav_state);
    _mav_put_uint8_t(buf, 81, arming_state);
    _mav_put_uint8_t(buf, 82, vehicle_type);
    _mav_put_uint8_t(buf, 83, estimator_valid);
    _mav_put_uint8_t(buf, 84, control_mode_flags);
    _mav_put_uint8_t(buf, 85, schema_version);
    _mav_put_float_array(buf, 24, q, 4);
    _mav_put_float_array(buf, 40, angular_velocity, 3);
    _mav_put_float_array(buf, 52, position_ned, 3);
    _mav_put_float_array(buf, 64, velocity_ned, 3);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_STATE, buf, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_CRC);
#else
    mavlink_cc_telemetry_state_t *packet = (mavlink_cc_telemetry_state_t *)msgbuf;
    packet->fc_timestamp_us = fc_timestamp_us;
    packet->sequence = sequence;
    packet->px4_boot_id = px4_boot_id;
    packet->mission_id = mission_id;
    packet->failsafe_flags = failsafe_flags;
    packet->heading = heading;
    packet->nav_state = nav_state;
    packet->arming_state = arming_state;
    packet->vehicle_type = vehicle_type;
    packet->estimator_valid = estimator_valid;
    packet->control_mode_flags = control_mode_flags;
    packet->schema_version = schema_version;
    mav_array_assign_float(packet->q, q, 4);
    mav_array_assign_float(packet->angular_velocity, angular_velocity, 3);
    mav_array_assign_float(packet->position_ned, position_ned, 3);
    mav_array_assign_float(packet->velocity_ned, velocity_ned, 3);
    _mav_finalize_message_chan_send(chan, MAVLINK_MSG_ID_CC_TELEMETRY_STATE, (const char *)packet, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_MIN_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_CRC);
#endif
}
#endif

#endif

// MESSAGE CC_TELEMETRY_STATE UNPACKING


/**
 * @brief Get field fc_timestamp_us from cc_telemetry_state message
 *
 * @return [us] FC monotonic time since PX4 boot.
 */
static inline uint64_t mavlink_msg_cc_telemetry_state_get_fc_timestamp_us(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint64_t(msg,  0);
}

/**
 * @brief Get field sequence from cc_telemetry_state message
 *
 * @return  Per-stream monotonic counter (wraps 2^32).
 */
static inline uint32_t mavlink_msg_cc_telemetry_state_get_sequence(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  8);
}

/**
 * @brief Get field px4_boot_id from cc_telemetry_state message
 *
 * @return  FC boot identity (constant per boot).
 */
static inline uint32_t mavlink_msg_cc_telemetry_state_get_px4_boot_id(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  12);
}

/**
 * @brief Get field mission_id from cc_telemetry_state message
 *
 * @return  Mission identity echoed from CC_MISSION_CONTEXT (0 if none).
 */
static inline uint32_t mavlink_msg_cc_telemetry_state_get_mission_id(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  16);
}

/**
 * @brief Get field failsafe_flags from cc_telemetry_state message
 *
 * @return  PX4 failsafe flags bitfield (as in failsafe_flags uORB).
 */
static inline uint32_t mavlink_msg_cc_telemetry_state_get_failsafe_flags(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint32_t(msg,  20);
}

/**
 * @brief Get field q from cc_telemetry_state message
 *
 * @return  Attitude quaternion (w, x, y, z), body to NED.
 */
static inline uint16_t mavlink_msg_cc_telemetry_state_get_q(const mavlink_message_t* msg, float *q)
{
    return _MAV_RETURN_float_array(msg, q, 4,  24);
}

/**
 * @brief Get field angular_velocity from cc_telemetry_state message
 *
 * @return [rad/s] Body angular rates (x, y, z).
 */
static inline uint16_t mavlink_msg_cc_telemetry_state_get_angular_velocity(const mavlink_message_t* msg, float *angular_velocity)
{
    return _MAV_RETURN_float_array(msg, angular_velocity, 3,  40);
}

/**
 * @brief Get field position_ned from cc_telemetry_state message
 *
 * @return [m] Local position NED.
 */
static inline uint16_t mavlink_msg_cc_telemetry_state_get_position_ned(const mavlink_message_t* msg, float *position_ned)
{
    return _MAV_RETURN_float_array(msg, position_ned, 3,  52);
}

/**
 * @brief Get field velocity_ned from cc_telemetry_state message
 *
 * @return [m/s] Local velocity NED.
 */
static inline uint16_t mavlink_msg_cc_telemetry_state_get_velocity_ned(const mavlink_message_t* msg, float *velocity_ned)
{
    return _MAV_RETURN_float_array(msg, velocity_ned, 3,  64);
}

/**
 * @brief Get field heading from cc_telemetry_state message
 *
 * @return [rad] Heading (yaw), NED, [-pi, pi].
 */
static inline float mavlink_msg_cc_telemetry_state_get_heading(const mavlink_message_t* msg)
{
    return _MAV_RETURN_float(msg,  76);
}

/**
 * @brief Get field nav_state from cc_telemetry_state message
 *
 * @return  PX4 nav_state (vehicle_status).
 */
static inline uint8_t mavlink_msg_cc_telemetry_state_get_nav_state(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  80);
}

/**
 * @brief Get field arming_state from cc_telemetry_state message
 *
 * @return  PX4 arming_state (vehicle_status).
 */
static inline uint8_t mavlink_msg_cc_telemetry_state_get_arming_state(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  81);
}

/**
 * @brief Get field vehicle_type from cc_telemetry_state message
 *
 * @return  PX4 vehicle type.
 */
static inline uint8_t mavlink_msg_cc_telemetry_state_get_vehicle_type(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  82);
}

/**
 * @brief Get field estimator_valid from cc_telemetry_state message
 *
 * @return  1 if local position/attitude estimates valid.
 */
static inline uint8_t mavlink_msg_cc_telemetry_state_get_estimator_valid(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  83);
}

/**
 * @brief Get field control_mode_flags from cc_telemetry_state message
 *
 * @return  Compact vehicle_control_mode bitfield.
 */
static inline uint8_t mavlink_msg_cc_telemetry_state_get_control_mode_flags(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  84);
}

/**
 * @brief Get field schema_version from cc_telemetry_state message
 *
 * @return  Payload schema version.
 */
static inline uint8_t mavlink_msg_cc_telemetry_state_get_schema_version(const mavlink_message_t* msg)
{
    return _MAV_RETURN_uint8_t(msg,  85);
}

/**
 * @brief Decode a cc_telemetry_state message into a struct
 *
 * @param msg The message to decode
 * @param cc_telemetry_state C-struct to decode the message contents into
 */
static inline void mavlink_msg_cc_telemetry_state_decode(const mavlink_message_t* msg, mavlink_cc_telemetry_state_t* cc_telemetry_state)
{
#if MAVLINK_NEED_BYTE_SWAP || !MAVLINK_ALIGNED_FIELDS
    cc_telemetry_state->fc_timestamp_us = mavlink_msg_cc_telemetry_state_get_fc_timestamp_us(msg);
    cc_telemetry_state->sequence = mavlink_msg_cc_telemetry_state_get_sequence(msg);
    cc_telemetry_state->px4_boot_id = mavlink_msg_cc_telemetry_state_get_px4_boot_id(msg);
    cc_telemetry_state->mission_id = mavlink_msg_cc_telemetry_state_get_mission_id(msg);
    cc_telemetry_state->failsafe_flags = mavlink_msg_cc_telemetry_state_get_failsafe_flags(msg);
    mavlink_msg_cc_telemetry_state_get_q(msg, cc_telemetry_state->q);
    mavlink_msg_cc_telemetry_state_get_angular_velocity(msg, cc_telemetry_state->angular_velocity);
    mavlink_msg_cc_telemetry_state_get_position_ned(msg, cc_telemetry_state->position_ned);
    mavlink_msg_cc_telemetry_state_get_velocity_ned(msg, cc_telemetry_state->velocity_ned);
    cc_telemetry_state->heading = mavlink_msg_cc_telemetry_state_get_heading(msg);
    cc_telemetry_state->nav_state = mavlink_msg_cc_telemetry_state_get_nav_state(msg);
    cc_telemetry_state->arming_state = mavlink_msg_cc_telemetry_state_get_arming_state(msg);
    cc_telemetry_state->vehicle_type = mavlink_msg_cc_telemetry_state_get_vehicle_type(msg);
    cc_telemetry_state->estimator_valid = mavlink_msg_cc_telemetry_state_get_estimator_valid(msg);
    cc_telemetry_state->control_mode_flags = mavlink_msg_cc_telemetry_state_get_control_mode_flags(msg);
    cc_telemetry_state->schema_version = mavlink_msg_cc_telemetry_state_get_schema_version(msg);
#else
        uint8_t len = msg->len < MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN? msg->len : MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN;
        memset(cc_telemetry_state, 0, MAVLINK_MSG_ID_CC_TELEMETRY_STATE_LEN);
    memcpy(cc_telemetry_state, _MAV_PAYLOAD(msg), len);
#endif
}
