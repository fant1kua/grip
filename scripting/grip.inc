#if defined _grip_included
	#endinput
#endif
#define _grip_included

#pragma reqlib grip
#if !defined AMXMODX_NOAUTOLOAD
	#pragma loadlib grip
#endif


enum GripRequestType
{
	GripRequestTypeGet,
	GripRequestTypePost,
	GripRequestTypePut,
	GripRequestTypeDelete
}

enum GripRequest
{
	Invalid_GripRequest = 0,
}

enum GripRequestOptionsHandle {
	Invalid_GripRequestOptionsHandle = 0,
}

/**
 * Starts sending of the request 
 * @note	The handle should look like:
 * 		public RequestHandler(GripResponseHandle:handle, const userData[], const userDataSize);
 *
 *
 * @param uri		Request URI. Supports TLS.
 * @param type		Request type which should be sended.
 * @param handler	A callback that will be called when request finishes execution.
 *
 * @noreturn
 */
native GripRequest:grip_request(const uri[], GripRequestType:type, const handler[], GripRequestOptionsHandle:options = GripRequestOptionsHandle, const userData[] = "", const userDataSize);

